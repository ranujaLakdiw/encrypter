//! Remembered sessions: keeping the session on this device under a nickname and a
//! passphrase, the list of what is kept, and opening one of them again.
//!
//! Only the sealed form (see `crate::keep`) ever reaches storage. The passphrase
//! is used once, to seal or to open, and is not kept anywhere.

use std::time::Duration;

use js_sys::{Object, Reflect};
use leptos::{ev, html, prelude::*, task::spawn_local};
use wasm_bindgen::JsValue;

use super::{compose::secret_input, dom, face, icon, sprite, ErrorBox, Pairing, Session, Ui};
use crate::keep::{self, KeepError, Kept, Key, Recipe};

/// Where the sealed sessions live in this browser's storage.
const STORE_KEY: &str = "huh:remembered-sessions";

/// How long a Forget button waits for its second press.
const ARMED_FOR: Duration = Duration::from_secs(4);

/// What the dialog is open for.
#[derive(Clone, PartialEq, Eq)]
enum Doing {
    /// Remembering the session in use right now.
    Save,
    /// Opening the remembered session with this nickname.
    Open(String),
}

#[derive(Clone, Copy)]
pub struct KeepUi {
    /// What is remembered on this device, as stored (so, still sealed).
    list: RwSignal<Vec<Kept>>,
    dialog: NodeRef<html::Dialog>,
    error: ErrorBox,
    doing: RwSignal<Doing>,
    nick_in: NodeRef<html::Input>,
    pass_in: NodeRef<html::Input>,
    again_in: NodeRef<html::Input>,
    pass_shown: RwSignal<bool>,
    /// While a passphrase is being stretched into a key, which takes a moment on purpose.
    busy: RwSignal<bool>,
    /// The nickname whose Forget was pressed once and is waiting for a second press.
    armed: RwSignal<Option<String>>,
    armed_timer: StoredValue<Option<TimeoutHandle>>,
    remember_btn: NodeRef<html::Button>,
}

impl KeepUi {
    pub fn new() -> Self {
        Self {
            list: RwSignal::new(Vec::new()),
            dialog: NodeRef::new(),
            error: ErrorBox::new(),
            doing: RwSignal::new(Doing::Save),
            nick_in: NodeRef::new(),
            pass_in: NodeRef::new(),
            again_in: NodeRef::new(),
            pass_shown: RwSignal::new(false),
            busy: RwSignal::new(false),
            armed: RwSignal::new(None),
            armed_timer: StoredValue::new(None),
            remember_btn: NodeRef::new(),
        }
    }

    fn disarm(self) {
        if let Some(timer) = self.armed_timer.get_value() {
            timer.clear();
        }
        self.armed_timer.set_value(None);
        self.armed.set(None);
    }

    /// Empties the dialog's fields, so no passphrase lingers in the page.
    fn clear_fields(self) {
        for node in [self.nick_in, self.pass_in, self.again_in] {
            if let Some(input) = node.get_untracked() {
                input.set_value("");
            }
        }
        self.pass_shown.set(false);
    }

    async fn close(self) {
        if let Some(dialog) = self.dialog.get_untracked() {
            dom::close_modal(dialog).await;
        }
        self.clear_fields();
        self.error.clear();
    }

    fn is_open(self) -> bool {
        self.dialog.get_untracked().is_some_and(|dialog| dialog.open())
    }

    /// The dialog has closed, by this page's doing or by the browser's own (a
    /// second Escape closes one whatever the page said about the first, even
    /// in the middle of stretching a passphrase). Either way the page behind
    /// it moves again and no passphrase is left sitting in its fields; work
    /// that was under way sees the dialog gone and stops.
    fn closed(self) {
        dom::modal_closed();
        self.clear_fields();
        self.error.clear();
    }
}

fn read() -> Vec<Kept> {
    dom::stored(STORE_KEY).map(|json| keep::list_from_json(&json)).unwrap_or_default()
}

fn write(list: &[Kept]) -> bool {
    dom::store(STORE_KEY, &keep::list_to_json(list))
}

/// Loads what is remembered, and keeps up with other tabs of this page.
pub fn start(ui: Ui) {
    refresh(ui);
    dom::on_window("storage", move |_| refresh(ui));
}

fn refresh(ui: Ui) {
    let list = read();
    // A session forgotten in another tab is no longer remembered in this one.
    let gone = ui.session.with_untracked(|s| {
        s.as_ref()
            .and_then(|s| s.nick.as_ref())
            .is_some_and(|nick| !list.iter().any(|kept| &kept.nick == nick))
    });
    if gone {
        ui.session.update(|s| {
            if let Some(session) = s {
                session.nick = None;
            }
        });
    }
    ui.keep.list.set(list);
}

fn focus_input(node: NodeRef<html::Input>) {
    if let Some(input) = node.get_untracked() {
        let _ = input.focus();
    }
}

/// What a passphrase field holds, as the same bytes however the keyboard put
/// its accents together.
fn passphrase(node: NodeRef<html::Input>) -> String {
    dom::nfc(&node.get_untracked().map(|input| input.value()).unwrap_or_default())
}

fn show(ui: Ui, doing: Doing) {
    let k = ui.keep;
    let Some(dialog) = k.dialog.get_untracked() else { return };
    if dialog.open() {
        return;
    }
    let saving = doing == Doing::Save;
    k.disarm();
    k.doing.set(doing);
    k.error.clear();
    k.busy.set(false);
    k.clear_fields();
    dom::show_modal(&dialog);
    // The fields this dialog is for are un-hidden by the state just set.
    dom::after_render(move || focus_input(if saving { k.nick_in } else { k.pass_in }));
}

fn cancel(ui: Ui) {
    let k = ui.keep;
    if !k.busy.get_untracked() {
        spawn_local(k.close());
    }
}

fn submit(ui: Ui) {
    let k = ui.keep;
    if k.busy.get_untracked() || !k.dialog.get_untracked().is_some_and(|dialog| dialog.open()) {
        return;
    }
    match k.doing.get_untracked() {
        Doing::Save => spawn_local(save(ui)),
        Doing::Open(nick) => spawn_local(open(ui, nick)),
    }
}

/// Stretches a passphrase into the key its recipe makes.
async fn stretch(pass: &str, recipe: &Recipe) -> Key {
    dom::stretch(pass, &recipe.salt, recipe.rounds).await
}

/// Seals the session in use under the passphrase and files it under the nickname.
async fn save(ui: Ui) {
    let k = ui.keep;
    let Some(secret) = ui.session.with_untracked(|s| s.as_ref().map(|session| session.secret)) else {
        k.error.show(face::DEAD, "Something went wrong", "There’s no session here to remember.");
        return;
    };

    let typed_nick = k.nick_in.get_untracked().map(|input| input.value()).unwrap_or_default();
    let Some(nick) = keep::tidy_nick(&typed_nick) else {
        focus_input(k.nick_in);
        if typed_nick.trim().is_empty() {
            k.error.show(face::WAVE, "Give it a nickname", "Whatever you’d like to call this session.");
        } else {
            k.error.show(
                face::GRUMP,
                "That nickname’s a bit much",
                &format!("Keep it to {} characters.", keep::NICK_MAX),
            );
        }
        return;
    };
    let pass = passphrase(k.pass_in);
    if pass.chars().count() < keep::PASS_MIN {
        focus_input(k.pass_in);
        k.error.show(
            face::WAVE,
            "Make the passphrase longer",
            &format!("At least {} characters. It’s all that guards this session here.", keep::PASS_MIN),
        );
        return;
    }
    if pass != passphrase(k.again_in) {
        focus_input(k.again_in);
        k.error.show(face::HUH, "Those don’t match", "Type the same passphrase in both boxes.");
        return;
    }
    let taken = |list: &[Kept]| {
        let taken = list.iter().find(|kept| keep::same_nick(&kept.nick, &nick))?;
        focus_input(k.nick_in);
        k.error.show(
            face::HUH,
            "That nickname’s taken",
            &format!("You already remember one as {}. Pick another nickname.", taken.nick),
        );
        Some(())
    };
    if taken(&read()).is_some() {
        return;
    }

    k.error.clear();
    k.busy.set(true);
    let recipe = Recipe::fresh();
    let key = stretch(&pass, &recipe).await;
    k.busy.set(false);
    if !k.is_open() {
        return; // closed while the passphrase was being stretched: nobody is asking any more
    }
    let sealed = keep::seal(&secret, &nick, &recipe, &key);

    // Read afresh: another tab may have saved or forgotten one in the meantime.
    let mut list = read();
    if taken(&list).is_some() {
        return;
    }
    list.push(Kept { nick: nick.clone(), saved: js_sys::Date::now() as u64, sealed });
    if !write(&list) {
        k.error.show(
            face::DEAD,
            "Couldn’t save it",
            "This browser isn’t letting the page keep anything. A private window won’t.",
        );
        return;
    }
    k.list.set(list);
    let remembered = nick.clone();
    ui.session.update(|s| {
        if let Some(session) = s {
            session.nick = Some(remembered);
        }
    });
    k.close().await;
    if let Some(name) = ui.session_name_el.get_untracked() {
        dom::focus(&name);
    }
    ui.announce(format!("Remembered as {nick}"));
}

/// Opens the remembered session called `nick` with the passphrase typed.
async fn open(ui: Ui, nick: String) {
    let k = ui.keep;
    let pass = passphrase(k.pass_in);
    if pass.is_empty() {
        focus_input(k.pass_in);
        k.error.show(face::WAVE, "Passphrase, please", "The one you chose when you remembered it.");
        return;
    }
    let Some(kept) = read().into_iter().find(|kept| kept.nick == nick) else {
        refresh(ui);
        k.error.show(face::HUH, "It’s gone", "That one has been forgotten since this page opened.");
        return;
    };

    let damaged = || {
        k.error.show(
            face::DEAD,
            "This one is damaged",
            "What was saved can’t be read any more. Forget it and pair again.",
        );
    };
    let Ok(recipe) = Recipe::of(&kept.sealed) else {
        damaged();
        return;
    };

    k.error.clear();
    k.busy.set(true);
    let key = stretch(&pass, &recipe).await;
    k.busy.set(false);
    if !k.is_open() {
        return; // closed while the passphrase was being stretched: the session stays shut
    }
    let opened = keep::open(&kept.sealed, &kept.nick, &key);
    match opened {
        // The first version of pairing, handed back its own two codes, made a
        // session whose key was all zeros: one that anybody could read. Nothing
        // makes such a session any more, but one may have been remembered.
        Ok(secret) if secret == [0; crate::kem::SECRET_LEN] => k.error.show(
            face::DEAD,
            "This session isn’t safe",
            "It was paired with its own codes, so anyone could read it. Forget it and pair again.",
        ),
        Ok(secret) => {
            k.close().await;
            // Nothing of an earlier pairing belongs to this session.
            ui.pairing.set_value(Pairing::default());
            ui.session.set(Some(Session::new(&secret, Some(kept.nick))));
            ui.error.clear();
            dom::after_render(move || {
                if let Some(name) = ui.session_name_el.get_untracked() {
                    dom::focus(&name);
                }
            });
            ui.announce(format!("Opened {nick}"));
        }
        Err(KeepError::WrongPass) => {
            if let Some(input) = k.pass_in.get_untracked() {
                let _ = input.focus();
                input.select();
            }
            k.error.show(face::GRUMP, "That’s not the passphrase", "Try again. It has to be exact.");
        }
        Err(KeepError::Damaged) => damaged(),
    }
}

/// Forgets a remembered session, on the second press: there is no getting it
/// back afterwards, so the first press only asks.
fn forget(ui: Ui, nick: &str) {
    let k = ui.keep;
    if k.armed.with_untracked(|armed| armed.as_deref() != Some(nick)) {
        k.disarm();
        k.armed.set(Some(nick.to_owned()));
        k.armed_timer
            .set_value(set_timeout_with_handle(move || k.disarm(), ARMED_FOR).ok());
        ui.announce(format!("Press again to forget {nick}"));
        return;
    }
    k.disarm();

    let mut list = read();
    list.retain(|kept| kept.nick != nick);
    if !write(&list) {
        ui.error.show(
            face::DEAD,
            "Couldn’t forget it",
            "This browser isn’t letting the page change what it keeps.",
        );
        return;
    }
    k.list.set(list);
    let in_use = ui.session.with_untracked(|s| s.as_ref().map(|s| s.nick.as_deref() == Some(nick)));
    if in_use == Some(true) {
        ui.session.update(|s| {
            if let Some(session) = s {
                session.nick = None;
            }
        });
    }
    // The button that was pressed is gone; hand the keyboard to what took its place.
    dom::after_render(move || {
        let next = if in_use.is_some() { k.remember_btn } else { ui.start_pairing_btn };
        if let Some(btn) = next.get_untracked() {
            dom::focus(&btn);
        }
    });
    ui.announce(format!("Forgot {nick}"));
}

/// "saved 18 Nov 2026"
fn saved_on(ms: u64) -> String {
    if ms == 0 {
        return "locked".to_owned();
    }
    let options = Object::new();
    for (part, style) in [("day", "numeric"), ("month", "short"), ("year", "numeric")] {
        let _ = Reflect::set(&options, &JsValue::from_str(part), &JsValue::from_str(style));
    }
    let day = js_sys::Date::new(&JsValue::from_f64(ms as f64)).to_locale_date_string("en-GB", &options);
    format!("saved {}", String::from(day))
}

/// The foot of the session card: remember the session in use, or forget it.
#[component]
pub fn KeepRow(ui: Ui) -> impl IntoView {
    let k = ui.keep;
    let nick = move || ui.session.with(|s| s.as_ref().and_then(|s| s.nick.clone()));
    let kept = move || ui.session.with(|s| s.as_ref().is_some_and(|s| s.nick.is_some()));
    let restored = move || ui.session.with(|s| s.as_ref().is_some_and(|s| s.restored));
    let armed = move || nick().is_some_and(|nick| k.armed.with(|armed| armed.as_ref() == Some(&nick)));
    let on_forget = move |_| {
        if let Some(nick) = nick() {
            forget(ui, &nick);
        }
    };

    view! {
        <div class="session-keep">
            <p id="keep-status" class="session-keep-text">
                <span hidden=kept>"Not saved. It’s gone when this tab closes."</span>
                <span hidden=move || !kept() || restored()>
                    "Remembered as "<strong>{move || nick().unwrap_or_default()}</strong>
                </span>
                <span hidden=move || !kept() || !restored()>"Kept on this device, under your passphrase."</span>
            </p>
            <button
                type="button"
                id="remember-session"
                class="btn btn-soft"
                hidden=kept
                node_ref=k.remember_btn
                on:click=move |_| show(ui, Doing::Save)
            >
                {icon("bookmark")}
                <span>"Remember"</span>
            </button>
            <button
                type="button"
                id="forget-session"
                class="btn"
                class:btn-ghost=move || !armed()
                class:btn-danger=armed
                hidden=move || !kept()
                on:click=on_forget
            >
                <span>{move || if armed() { "Really forget?" } else { "Forget" }}</span>
            </button>
        </div>
    }
}

fn kept_row(ui: Ui, kept: Kept) -> impl IntoView {
    let k = ui.keep;
    let nick = StoredValue::new(kept.nick.clone());
    let armed = move || k.armed.with(|armed| nick.with_value(|nick| armed.as_ref() == Some(nick)));

    view! {
        <li class="kept-row">
            {icon("lock")}
            <div class="kept-info">
                <p class="kept-nick">{kept.nick.clone()}</p>
                <p class="meta mono">{saved_on(kept.saved)}</p>
            </div>
            <button
                type="button"
                class="btn btn-soft kept-open"
                aria-label=format!("Open {}", kept.nick)
                on:click=move |_| show(ui, Doing::Open(nick.get_value()))
            >
                <span>"Open"</span>
            </button>
            <button
                type="button"
                class="btn kept-forget"
                class:btn-ghost=move || !armed()
                class:btn-danger=armed
                aria-label=move || {
                    let nick = nick.get_value();
                    if armed() { format!("Really forget {nick}?") } else { format!("Forget {nick}") }
                }
                on:click=move |_| forget(ui, &nick.get_value())
            >
                <svg aria-hidden="true" class="icon" hidden=armed inner_html=sprite("x")></svg>
                <span hidden=move || !armed()>"Forget?"</span>
            </button>
        </li>
    }
}

/// The sessions remembered on this device, each waiting for its passphrase.
/// Shown while no session is in use.
#[component]
pub fn KeptList(ui: Ui) -> impl IntoView {
    let k = ui.keep;
    let nothing_to_show = move || k.list.with(|list| list.is_empty()) || ui.session.with(|s| s.is_some());

    view! {
        <div id="kept" class="kept" hidden=nothing_to_show>
            <p id="kept-label" class="eyebrow">"Remembered on this device"</p>
            <ul class="kept-list" aria-labelledby="kept-label">
                <For
                    each=move || k.list.get()
                    key=|kept| (kept.nick.clone(), kept.saved, kept.sealed.clone())
                    children=move |kept| kept_row(ui, kept)
                />
            </ul>
        </div>
    }
}

/// One dialog for both ends of remembering: naming and locking the session in
/// use, and unlocking one that was remembered before.
#[component]
pub fn KeepDialog(ui: Ui) -> impl IntoView {
    let k = ui.keep;
    let saving = move || k.doing.with(|doing| *doing == Doing::Save);
    let title = move || {
        k.doing.with(|doing| match doing {
            Doing::Save => "Remember this session".to_owned(),
            Doing::Open(nick) => format!("Open {nick}"),
        })
    };
    let submit_label = move || match (saving(), k.busy.get()) {
        (true, false) => "Remember",
        (true, true) => "Locking…",
        (false, false) => "Open",
        (false, true) => "Opening…",
    };
    let on_esc = move |e: ev::Event| {
        e.prevent_default(); // we close with our own animation
        cancel(ui);
    };
    let on_submit = move |e: ev::SubmitEvent| {
        e.prevent_default();
        submit(ui);
    };

    view! {
        <dialog
            id="keep-dialog"
            class="dialog"
            aria-labelledby="keep-title"
            aria-describedby="keep-desc"
            node_ref=k.dialog
            on:cancel=on_esc
            on:close=move |_| k.closed()
        >
            <form class="dialog-form" novalidate on:submit=on_submit>
                <div class="dialog-head">
                    <h2 id="keep-title">{title}</h2>
                    <p id="keep-desc" class="note">
                        {move || {
                            if saving() {
                                "It stays in this browser, locked with a passphrase only you know. Nothing is sent anywhere."
                            } else {
                                "Type the passphrase you locked it with."
                            }
                        }}
                    </p>
                </div>
                <div class="field" hidden=move || !saving()>
                    <label for="keep-nick" class="eyebrow">"Nickname"</label>
                    <input
                        id="keep-nick"
                        class="text-input"
                        type="text"
                        maxlength=keep::NICK_MAX.to_string()
                        autocomplete="off"
                        spellcheck="false"
                        placeholder="What you’ll call it"
                        node_ref=k.nick_in
                        on:input=move |_| k.error.clear()
                    />
                </div>
                <div class="field">
                    <label for="keep-pass" class="eyebrow">"Passphrase"</label>
                    {secret_input("keep-pass", "Just for this device", "passphrase", k.pass_shown, k.pass_in, move || k.error.clear())}
                </div>
                <div class="field" hidden=move || !saving()>
                    <label for="keep-again" class="eyebrow">"Passphrase, once more"</label>
                    {secret_input("keep-again", "Same again", "passphrase", k.pass_shown, k.again_in, move || k.error.clear())}
                    <p class="hint">
                        {format!(
                            "At least {} characters. Nobody can recover it for you, so pick one you’ll remember.",
                            keep::PASS_MIN,
                        )}
                    </p>
                </div>
                {k.error.view("keep-error")}
                <div class="dialog-actions">
                    <button type="button" id="keep-cancel-btn" class="btn btn-secondary" on:click=move |_| cancel(ui)>
                        <span>"Cancel"</span>
                    </button>
                    <button type="submit" id="keep-submit-btn" class="btn btn-primary" disabled=move || k.busy.get()>
                        <span>{submit_label}</span>
                    </button>
                </div>
            </form>
        </dialog>
    }
}
