//! Compose: the message box (or an attached file), how to lock it, and the two big buttons.

use std::rc::Rc;

use leptos::{ev, html, prelude::*, task::spawn_local};
use wasm_bindgen::JsValue;

use super::{
    dom, face, fmt_size, icon, pair,
    result::{self, show_huh_file, show_text, show_unlocked_file, How, Old},
    sprite, Attached, Mode, Step, Ui, MAX_FILE_BYTES, MAX_HUH_BYTES,
};
use crate::{
    b64,
    envelope::{self, Kind},
    huh,
    kem::Secret,
    locker::{self, Needs, Prepared, UnlockError, Unlocked},
};

pub fn drag_has_files(e: &web_sys::DragEvent) -> bool {
    e.data_transfer()
        .is_some_and(|dt| dt.types().includes(&JsValue::from_str("Files"), 0))
}

/// What the mode in use locks and unlocks with.
#[derive(Clone)]
enum Using {
    /// The password, as it was typed.
    Password(String),
    /// The secret of the session in use.
    Session(Secret),
}

/// What one lock or unlock is done with, settled when its button is pressed.
/// Stretching a password takes a moment, and the mode or the session can be
/// changed in that moment: the work carries on with what it started with, and
/// the result says so.
#[derive(Clone)]
struct Chosen {
    using: Using,
    /// "with a password", "in amber-arc-07".
    via: String,
}

/// Returns what the current mode locks or unlocks with, or `None` after showing why not.
fn current_key(ui: Ui, encrypting: bool) -> Option<Chosen> {
    let chosen = |using| Some(Chosen { using, via: ui.via() });
    match ui.mode.get_untracked() {
        Mode::Password => {
            let input = ui.pass_in.get_untracked()?;
            let pass = input.value();
            if pass.is_empty() {
                let _ = input.focus();
                ui.error.show(
                    face::WAVE,
                    "Password, please",
                    if encrypting {
                        "Add the one you both know, or switch to Session."
                    } else {
                        "Type the one it was locked with."
                    },
                );
                return None;
            }
            // For locking only: whatever was locked with a shorter one still has to open.
            if encrypting && pass.chars().count() < envelope::PASS_MIN {
                let _ = input.focus();
                ui.error.show(
                    face::WAVE,
                    "Make the password longer",
                    &format!(
                        "At least {} characters. Anyone who gets hold of the huH can keep guessing at it.",
                        envelope::PASS_MIN
                    ),
                );
                return None;
            }
            chosen(Using::Password(pass))
        }
        Mode::Session => match ui.session.get_untracked() {
            Some(session) => chosen(Using::Session(session.secret)),
            None => {
                ui.error.show(
                    face::ALARM,
                    "No session yet",
                    if encrypting {
                        "Pair with someone first, then lock away."
                    } else {
                        "Pair with someone first, then unlock away."
                    },
                );
                None
            }
        },
    }
}

fn detach_file(ui: Ui) {
    if let Some(url) = ui.attached.get_untracked().and_then(|a| a.thumb) {
        dom::revoke_url(&url);
    }
    ui.attached.set(None);
}

/// Takes a picked / dropped / pasted file. A .huh (or any text file that is only
/// h, u, H and U) is held as a huH to unlock; anything else is a file to lock.
fn attach_file(ui: Ui, file: Option<web_sys::File>) {
    let Some(file) = file else { return };
    ui.error.clear();
    spawn_local(async move {
        let (name, mime, size) = (file.name(), file.type_(), file.size());

        let lower = name.to_lowercase();
        let maybe_huh = lower.ends_with(".huh") || lower.ends_with(".txt") || mime == "text/plain";
        let mut huh_text: Option<Rc<str>> = None;
        if maybe_huh && size <= MAX_HUH_BYTES {
            if let Ok(text) = dom::file_text(&file).await {
                if huh::looks_like_huh(&text) {
                    huh_text = Some(huh::js_trim(&text).into());
                }
            }
        }
        if huh_text.is_none() && size > MAX_FILE_BYTES {
            ui.error.show(
                face::GRUMP,
                "That file’s a bit much",
                &format!(
                    "Files up to {} only. huH makes them five times bigger, so keep it small.",
                    fmt_size(MAX_FILE_BYTES)
                ),
            );
            return;
        }

        detach_file(ui);
        let thumb = if mime.starts_with("image/") { dom::object_url(&file) } else { None };
        let meta = if huh_text.is_some() {
            format!("{} of huH · ready to unlock", fmt_size(size))
        } else if mime.is_empty() {
            format!("{} · ready to lock", fmt_size(size))
        } else {
            format!("{} · {mime} · ready to lock", fmt_size(size))
        };
        ui.announce(format!("Attached {name}"));
        ui.attached.set(Some(Attached { file, name, mime, meta, huh: huh_text, thumb }));
    });
}

fn clear_text(ui: Ui) {
    detach_file(ui);
    ui.error.clear();
    ui.text_is_huh.set(false);
    if let Some(text) = ui.text_in.get_untracked() {
        text.set_value("");
        // The box is un-hidden by the detach; it can take the caret once that lands.
        dom::after_render(move || {
            let _ = text.focus();
        });
    }
}

/// For a lock that was asked for and could not be done. Whatever the result
/// pane was showing goes, too: left there, the last huH would look like the
/// answer to this one, ready to be copied and sent.
fn something_went_wrong(ui: Ui, why: &str) {
    result::clear(ui);
    ui.error.show(face::DEAD, "Something went wrong", why);
}

/// The second step of every lock, and its ending: encrypt, then show the huH.
async fn seal_and_show(ui: Ui, prepared: Prepared, chosen: Chosen, file_name: Option<String>) {
    let began = ui.enter(Step::Encrypt).await;
    let huh = match chosen.using {
        Using::Password(pass) => {
            // Stretching the password into a key is most of the wait, and it is
            // meant to be: it is what makes guessing at a huH slow.
            let salt = envelope::fresh_salt();
            let key = dom::stretch(&dom::nfc(&pass), &salt, envelope::PASSWORD_ROUNDS).await;
            prepared.seal_stretched(salt, &key)
        }
        Using::Session(secret) => prepared.seal_session(&secret),
    };
    ui.linger(began).await;
    match file_name {
        Some(name) => show_huh_file(ui, &huh, format!("{name}.huh"), chosen.via),
        None => show_text(ui, &huh, true, How { via: chosen.via, old: None }),
    }
}

/// Says why a huH didn't unlock. `with_password` is what was tried on it.
/// Whatever the result pane was showing goes: left there, the last thing
/// unlocked would look like what this huH said.
fn show_unlock_error(ui: Ui, error: UnlockError, with_password: bool) {
    result::clear(ui);
    match error {
        UnlockError::NotHuh => ui.error.show(
            face::HUH,
            "That isn’t a huH",
            "Paste the whole thing. It’s only h, u, H and U.",
        ),
        // It opened, so the key was right; it is what was locked into it that
        // makes no sense. (A huH that was cut short or changed on the way doesn't
        // get this far any more: that is a wrong key, below.)
        UnlockError::Damaged => ui.error.show(
            face::DEAD,
            "This huH is damaged",
            "It opened, but what’s inside can’t be read as a message or a file. Ask for it to be locked and sent again.",
        ),
        // The first letters of a huH say what kind it is, and nothing vouches
        // for them until it opens. Anyone could make a huH look "newer", so this
        // must not send people off to reload (and lose a session) on its say-so.
        UnlockError::Newer => ui.error.show(
            face::HUH,
            "This huH won’t open here",
            "Its first letters have changed on the way, or a newer version of this page made it. If this page has been open for days, reload it (a session you haven’t remembered ends when you do).",
        ),
        UnlockError::NeedsSession => ui.error.show(
            face::WAVE,
            "That one’s locked in a session",
            "Switch to Session to unlock it.",
        ),
        UnlockError::NeedsPassword => ui.error.show(
            face::WAVE,
            "That one’s locked with a password",
            "Switch to Password and type the one it was locked with.",
        ),
        // A huH that was changed on its way looks the same from here as one
        // locked with something else: it simply doesn't open.
        UnlockError::WrongKey if with_password => ui.error.show(
            face::GRUMP,
            "Invalid password, mate!",
            "You frogot it didn’t you. (Or the huH got changed on its way here.)",
        ),
        UnlockError::WrongKey => ui.error.show(
            face::GRUMP,
            "Session keys don’t match",
            "This huH was locked in another session, or got changed on its way here.",
        ),
    }
}

/// Locks the attached file: header + bytes → compress → encrypt → huH → a .huh to save.
async fn encrypt_attached(ui: Ui, attached: Attached) {
    let Some(key) = current_key(ui, true) else { return result::clear(ui) };
    let began = ui.enter(Step::Compress).await;
    let prepared = match dom::file_bytes(&attached.file).await {
        Ok(bytes) => locker::prepare_file(&bytes, &attached.name, &attached.mime).map_err(|e| e.message().to_owned()),
        Err(e) => Err(dom::js_message(&e).unwrap_or_else(|| "An unknown error occurred.".to_owned())),
    };
    ui.linger(began).await;
    match prepared {
        Ok(prepared) => seal_and_show(ui, prepared, key, Some(attached.name)).await,
        Err(why) => something_went_wrong(ui, &why),
    }
    ui.job.set(None);
}

/// Locks what is in the compose box: compress, then encrypt.
async fn encrypt(ui: Ui) {
    if ui.job.get_untracked().is_some() {
        return;
    }
    if let Some(attached) = ui.attached.get_untracked() {
        encrypt_attached(ui, attached).await;
        return;
    }
    let Some(input) = ui.text_in.get_untracked() else { return };
    let raw = dom::raw_value(&input);
    let msg = String::from(&raw);
    if huh::js_trim(&msg).is_empty() {
        let _ = input.focus();
        ui.error.show(
            face::ALARM,
            "Enter something first…",
            "I mean, we can encrypt nothing, and that’s nothing.",
        );
        return;
    }
    // (With something to lock and nothing to lock it with, the last result is
    // out of date all the same: it goes.)
    let Some(key) = current_key(ui, true) else { return result::clear(ui) };
    // Half a surrogate pair can't be stored as text; refuse it rather than swap it for "�".
    if !raw.is_valid_utf16() {
        something_went_wrong(ui, "There’s a broken character in that text.");
        return;
    }
    let began = ui.enter(Step::Compress).await;
    let prepared = locker::prepare_text(&msg);
    ui.linger(began).await;
    match prepared {
        Ok(prepared) => seal_and_show(ui, prepared, key, None).await,
        Err(e) => something_went_wrong(ui, e.message()),
    }
    ui.job.set(None);
}

/// Unlocks what is in the compose box: decrypt, then decompress if it was squeezed.
async fn decrypt(ui: Ui) {
    if ui.job.get_untracked().is_some() {
        return;
    }
    let msg: Rc<str> = match ui.attached.get_untracked() {
        Some(attached) => match attached.huh {
            Some(text) => text,
            None => {
                ui.error.show(
                    face::HUH,
                    "That isn’t a huH",
                    "Only a .huh can be unlocked. Hit Encrypt to lock this file instead.",
                );
                return;
            }
        },
        None => {
            let Some(input) = ui.text_in.get_untracked() else { return };
            let value = input.value();
            let trimmed = huh::js_trim(&value);
            if trimmed.is_empty() {
                let _ = input.focus();
                ui.error.show(
                    face::ALARM,
                    "Enter something first…",
                    "Why waste processing power on an empty string?",
                );
                return;
            }
            trimmed.into()
        }
    };
    // What can be told at a glance is said now, before a password or a session
    // is asked for that could never open it: that this is no huH at all, or that
    // it was locked the other way (today's huHs say how they were locked).
    let with_password = ui.mode.get_untracked() == Mode::Password;
    match (locker::glance(&msg), with_password) {
        (Err(e), _) => return show_unlock_error(ui, e, with_password),
        (Ok(Some(Kind::Session)), true) => return show_unlock_error(ui, UnlockError::NeedsSession, true),
        (Ok(Some(Kind::Password)), false) => return show_unlock_error(ui, UnlockError::NeedsPassword, false),
        _ => {}
    }
    let Some(Chosen { using, via }) = current_key(ui, false) else { return result::clear(ui) };

    let began = ui.enter(Step::Decrypt).await;
    let opened = match locker::read(&msg) {
        Ok(sealed) => match (sealed.needs(), &using) {
            // An old huH was locked with the password just as it was typed, or
            // with the session's key written out as text.
            (Needs::Old, Using::Password(pass)) => sealed.open_old(pass),
            (Needs::Old, Using::Session(secret)) => sealed.open_old(&b64::encode(secret)),
            (Needs::Password { salt, rounds }, Using::Password(pass)) => {
                let key = dom::stretch(&dom::nfc(pass), &salt, rounds).await;
                sealed.open_stretched(&key)
            }
            (Needs::Session, Using::Session(secret)) => sealed.open_session(secret),
            (Needs::Password { .. }, Using::Session(_)) => Err(UnlockError::NeedsPassword),
            (Needs::Session, Using::Password(_)) => Err(UnlockError::NeedsSession),
        },
        Err(e) => Err(e),
    };
    ui.linger(began).await;
    // An old huH carries no check, so what it says can't be vouched for; the
    // result says so, and what that means for the way it was locked.
    let with_password = matches!(using, Using::Password(_));
    let old = opened
        .as_ref()
        .is_ok_and(|opened| opened.is_old())
        .then_some(if with_password { Old::Password } else { Old::Session });
    let unlocked = match opened {
        Ok(opened) if opened.is_squeezed() => {
            let began = ui.enter(Step::Decompress).await;
            let unlocked = opened.finish();
            ui.linger(began).await;
            unlocked
        }
        Ok(opened) => opened.finish(),
        Err(e) => Err(e),
    };
    let how = How { via, old };
    match unlocked {
        Ok(Unlocked::Text(text)) => show_text(ui, &text, false, how),
        Ok(Unlocked::File { name, mime, bytes }) => show_unlocked_file(ui, &name, &mime, &bytes, how),
        Err(e) => show_unlock_error(ui, e, with_password),
    }
    ui.job.set(None);
}

/// A password-style field with its show/hide toggle.
pub fn secret_input(
    id: &'static str,
    placeholder: &'static str,
    what: &'static str,
    shown: RwSignal<bool>,
    node: NodeRef<html::Input>,
    on_input: impl Fn() + 'static,
) -> impl IntoView {
    view! {
        <div class="pw-wrap">
            <input
                id=id
                type=move || if shown.get() { "text" } else { "password" }
                autocomplete="off"
                spellcheck="false"
                placeholder=placeholder
                node_ref=node
                on:input=move |_| on_input()
            />
            <button
                type="button"
                class="pw-toggle"
                data-for=id
                aria-label=move || format!("{} {what}", if shown.get() { "Hide" } else { "Show" })
                on:click=move |_| shown.update(|s| *s = !*s)
            >
                <svg aria-hidden="true" class="icon icon-hidden" hidden=move || shown.get() inner_html=sprite("eye-off")></svg>
                <svg aria-hidden="true" class="icon icon-shown" hidden=move || !shown.get() inner_html=sprite("eye")></svg>
            </button>
        </div>
    }
}

#[component]
pub fn Compose(ui: Ui) -> impl IntoView {
    let file_input = NodeRef::<html::Input>::new();
    let pass_shown = RwSignal::new(false);
    let dragging = RwSignal::new(false);
    let drag_depth = StoredValue::new(0i32); // enter/leave fire for every child, so count them

    let attached = move |f: fn(&Attached) -> bool| ui.attached.with(|a| a.as_ref().is_some_and(f));
    let has_file = move || ui.attached.with(|a| a.is_some());
    let has_thumb = move || attached(|a| a.thumb.is_some());
    // A huH in the box (typed or attached) makes Decrypt the main button.
    let is_huh = move || {
        ui.attached
            .with(|a| a.as_ref().map(|a| a.huh.is_some()))
            .unwrap_or_else(|| ui.text_is_huh.get())
    };
    let attached_text =
        move |f: fn(&Attached) -> String| ui.attached.with(|a| a.as_ref().map(f).unwrap_or_default());
    // While something is being locked or unlocked, both buttons wait and the one
    // that was pressed says which step it is on.
    let busy = move || ui.job.get().is_some();
    let lock_label = move || ui.job.get().filter(|s| s.locking()).map_or("Encrypt", Step::label);
    let unlock_label = move || ui.job.get().filter(|s| !s.locking()).map_or("Decrypt", Step::label);

    let on_text_input = move |_| {
        ui.error.clear();
        if let Some(text) = ui.text_in.get_untracked() {
            ui.text_is_huh.set(huh::looks_like_huh(&text.value()));
        }
    };

    // Files in: the picker, a drop onto the compose card, or a paste into the message box.
    let on_pick = move |_| {
        if let Some(input) = file_input.get_untracked() {
            attach_file(ui, input.files().and_then(|f| f.get(0)));
            input.set_value("");
        }
    };
    let on_paste = move |e: ev::ClipboardEvent| {
        let file = e.clipboard_data().and_then(|d| d.files()).and_then(|f| f.get(0));
        if file.is_some() {
            e.prevent_default();
            attach_file(ui, file);
        }
    };
    let on_drag_enter = move |e: ev::DragEvent| {
        if !drag_has_files(&e) {
            return;
        }
        e.prevent_default();
        drag_depth.update_value(|d| *d += 1);
        dragging.set(true);
    };
    let on_drag_over = move |e: ev::DragEvent| {
        if !drag_has_files(&e) {
            return;
        }
        e.prevent_default();
        if let Some(dt) = e.data_transfer() {
            dt.set_drop_effect("copy");
        }
    };
    let on_drag_leave = move |e: ev::DragEvent| {
        if !drag_has_files(&e) {
            return;
        }
        drag_depth.update_value(|d| *d -= 1);
        if drag_depth.get_value() <= 0 {
            drag_depth.set_value(0);
            dragging.set(false);
        }
    };
    let on_drop = move |e: ev::DragEvent| {
        if !drag_has_files(&e) {
            return;
        }
        e.prevent_default();
        drag_depth.set_value(0);
        dragging.set(false);
        attach_file(ui, e.data_transfer().and_then(|d| d.files()).and_then(|f| f.get(0)));
    };

    let on_remove = move |_| {
        detach_file(ui);
        ui.error.clear();
        if let Some(text) = ui.text_in.get_untracked() {
            dom::after_render(move || {
                let _ = text.focus();
            });
        }
    };

    let set_mode = move |mode: Mode| {
        ui.mode.set(mode);
        ui.error.clear();
    };

    view! {
        <section
            class="compose card"
            aria-label="Compose"
            class:dragging=move || dragging.get()
            on:dragenter=on_drag_enter
            on:dragover=on_drag_over
            on:dragleave=on_drag_leave
            on:drop=on_drop
        >
            <div class="field">
                <div class="field-head">
                    <div class="field-label-row">
                        <label for="text" class="eyebrow">"Message"</label>
                        <span id="huh-chip" class="chip mono" hidden=move || !is_huh()>
                            {move || if has_file() { "huH file" } else { "huH detected" }}
                        </span>
                    </div>
                    <div class="field-actions">
                        <button
                            type="button"
                            id="attach-btn"
                            class="btn btn-ghost"
                            on:click=move |_| {
                                if let Some(input) = file_input.get_untracked() {
                                    input.click();
                                }
                            }
                        >
                            {icon("clip")}
                            <span>"Attach"</span>
                        </button>
                        <input type="file" id="file-input" hidden aria-label="Attach a file" node_ref=file_input on:change=on_pick/>
                        <button type="button" id="clear" class="btn btn-ghost" on:click=move |_| clear_text(ui)>
                            "Clear"
                        </button>
                    </div>
                </div>
                <textarea
                    id="text"
                    rows="7"
                    placeholder="Type a message, paste a huH to read it, or drop a file here"
                    spellcheck="false"
                    class:huh=move || !has_file() && is_huh()
                    hidden=has_file
                    node_ref=ui.text_in
                    on:input=on_text_input
                    on:paste=on_paste
                ></textarea>
                // Attached file: takes the message box's place.
                <div id="attach-card" class="attach-card" hidden=move || !has_file()>
                    <img
                        id="attach-thumb"
                        class="attach-thumb"
                        alt=""
                        hidden=move || !has_thumb()
                        src=move || ui.attached.with(|a| a.as_ref().and_then(|a| a.thumb.clone()))
                    />
                    <div id="attach-glyph" class="attach-glyph mono" aria-hidden="true" hidden=has_thumb>
                        <span id="attach-glyph-text" hidden=move || attached(|a| a.huh.is_none())>"huH"</span>
                        <svg
                            class="icon"
                            id="attach-glyph-icon"
                            hidden=move || !attached(|a| a.huh.is_none())
                            inner_html=sprite("file")
                        ></svg>
                    </div>
                    <div class="attach-info">
                        <p id="attach-name" class="attach-name">{move || attached_text(|a| a.name.clone())}</p>
                        <p id="attach-meta" class="meta mono">{move || attached_text(|a| a.meta.clone())}</p>
                    </div>
                    <button type="button" id="attach-remove" class="btn btn-ghost" aria-label="Remove file" on:click=on_remove>
                        {icon("x")}
                    </button>
                </div>
            </div>

            <div class="field">
                <span id="mode-label" class="eyebrow">
                    {move || if is_huh() { "Decrypt with" } else { "Encrypt with" }}
                </span>
                <div role="group" aria-labelledby="mode-label" class="segmented">
                    <button
                        type="button"
                        id="mode-password"
                        class="seg"
                        aria-pressed=move || if ui.mode.get() == Mode::Password { "true" } else { "false" }
                        on:click=move |_| set_mode(Mode::Password)
                    >
                        {icon("key")}
                        <span>"Password"</span>
                    </button>
                    <button
                        type="button"
                        id="mode-session"
                        class="seg"
                        aria-pressed=move || if ui.mode.get() == Mode::Session { "true" } else { "false" }
                        on:click=move |_| set_mode(Mode::Session)
                    >
                        {icon("session")}
                        <span>"Session"</span>
                    </button>
                </div>
            </div>

            // Password mode
            <div id="panel-password" class="field" hidden=move || ui.mode.get() != Mode::Password>
                <label for="pass" class="eyebrow">"Shared password"</label>
                {secret_input("pass", "The one you both know", "password", pass_shown, ui.pass_in, move || ui.error.clear())}
                <p class="hint">"They’ll need this exact password to read it."</p>
            </div>

            // Session mode
            <div id="panel-session" hidden=move || ui.mode.get() != Mode::Session>
                <pair::SessionPanel ui/>
            </div>

            <div class="actions">
                <button
                    type="button"
                    id="encrypt-btn"
                    class="btn btn-big"
                    class:btn-primary=move || !is_huh()
                    class:btn-secondary=is_huh
                    class:busy=move || ui.job.get().is_some_and(|step| step.locking())
                    disabled=busy
                    on:click=move |_| spawn_local(encrypt(ui))
                >
                    {icon("lock")}
                    <span>{lock_label}</span>
                </button>
                <button
                    type="button"
                    id="decrypt-btn"
                    class="btn btn-big"
                    class:btn-primary=is_huh
                    class:btn-secondary=move || !is_huh()
                    class:busy=move || ui.job.get().is_some_and(|step| !step.locking())
                    disabled=busy
                    on:click=move |_| spawn_local(decrypt(ui))
                >
                    {icon("unlock")}
                    <span>{unlock_label}</span>
                </button>
            </div>

            {ui.error.view("error")}
        </section>
    }
}
