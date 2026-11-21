//! Session pairing: the panel in the compose card and the dialog that trades the two codes.

use std::{rc::Rc, time::Duration};

use js_sys::JsString;
use leptos::{ev, html, prelude::*, task::spawn_local};

use super::{
    compose::secret_input, dom, face, icon, remember, Answer, ErrorBox, Flash, Pairing, Session, Ui,
};
use crate::{
    b64, huh,
    kem::{self, KeyPair, PairError},
};

/// What the line beside "Copy my reply" says while the reply is being made.
const GENERATING: &str = "Generating…";

/// What the open dialog needs to remember.
#[derive(Default)]
struct DialogState {
    open: bool,
    is_edit: bool,
    /// My answer from before the dialog opened, to put back on cancel.
    orig_answer: Option<Rc<Answer>>,
    timer: Option<TimeoutHandle>,
    /// Bumped whenever their key changes or the dialog closes, so a reply still
    /// being made for an earlier key is dropped.
    generation: u32,
}

#[derive(Clone, Copy)]
pub struct PairUi {
    pub dialog: NodeRef<html::Dialog>,
    error: ErrorBox,
    title: RwSignal<&'static str>,
    start_label: RwSignal<&'static str>,
    other_pk: NodeRef<html::Input>,
    other_ct: NodeRef<html::Input>,
    pk_shown: RwSignal<bool>,
    ct_shown: RwSignal<bool>,
    copy_pk: NodeRef<html::Button>,
    reply_ready: RwSignal<bool>,
    ct_status: RwSignal<&'static str>,
    pk_flash: Flash,
    ct_flash: Flash,
    key_flash: Flash,
    reply_flash: Flash,
    state: StoredValue<DialogState, LocalStorage>,
}

impl PairUi {
    pub fn new() -> Self {
        Self {
            dialog: NodeRef::new(),
            error: ErrorBox::new(),
            title: RwSignal::new("Pair a session"),
            start_label: RwSignal::new("Pair"),
            other_pk: NodeRef::new(),
            other_ct: NodeRef::new(),
            pk_shown: RwSignal::new(false),
            ct_shown: RwSignal::new(false),
            copy_pk: NodeRef::new(),
            reply_ready: RwSignal::new(false),
            ct_status: RwSignal::new("Needs their key first"),
            pk_flash: Flash::new(),
            ct_flash: Flash::new(),
            key_flash: Flash::new(),
            reply_flash: Flash::new(),
            state: StoredValue::new_local(DialogState::default()),
        }
    }

    fn enable_reply(self) {
        self.reply_ready.set(true);
        self.ct_status.set("Ready to send");
    }

    fn disable_reply(self, status: &'static str) {
        self.reply_ready.set(false);
        self.ct_status.set(status);
    }

    fn field(self, node: NodeRef<html::Input>) -> String {
        node.get_untracked()
            .map(|input| huh::js_trim(&input.value()).to_owned())
            .unwrap_or_default()
    }

    /// Stops listening to the dialog: no pending reply survives, and a second
    /// click on Pair or Cancel while it animates shut does nothing.
    fn cleanup(self) {
        self.state.update_value(|s| {
            s.open = false;
            s.generation += 1;
            if let Some(timer) = s.timer.take() {
                timer.clear();
            }
        });
    }

    async fn close(self) {
        if let Some(dialog) = self.dialog.get_untracked() {
            dom::close_modal(dialog).await;
        }
    }
}

fn public_key(ui: Ui) -> Option<JsString> {
    ui.pairing
        .with_value(|p| p.keypair.as_ref().map(|k| JsString::from(b64::encode(k.public()))))
}

fn my_reply(ui: Ui) -> Option<JsString> {
    ui.pairing
        .with_value(|p| p.answer.as_ref().map(|a| JsString::from(b64::encode(&a.reply))))
}

/// Sees to my reply whenever the "their key" box changes.
///
/// A reply is made once for a key and then kept. The other person may already
/// be holding it, and a second reply to the same key would be a different one:
/// the two of them would end up in sessions that don't match. So a key pasted
/// again, or put back as it was after a slip of the keyboard, gets the reply it
/// already has. Only a different key gets a new one, and until that is ready
/// Pair's "Not ready yet" guard holds.
fn on_their_key(ui: Ui) {
    let d = ui.pair;
    d.error.clear();
    let mine = d
        .state
        .try_update_value(|s| {
            if let Some(timer) = s.timer.take() {
                timer.clear();
            }
            s.generation += 1;
            s.generation
        })
        .unwrap_or_default();

    let their_key = d.field(d.other_pk);
    if their_key.is_empty() {
        d.disable_reply("Needs their key first");
        return;
    }
    if ui.pairing.with_value(|p| p.answer.as_ref().is_some_and(|a| a.their_key == their_key)) {
        d.enable_reply();
        return;
    }
    d.disable_reply(GENERATING);
    let timer = set_timeout_with_handle(
        move || {
            if d.state.with_value(|s| s.generation != mine) {
                return; // superseded meanwhile
            }
            // Their key must be somebody else's. Pairing with a copy of your own
            // codes is a session with nobody in it.
            let own = |key: &[u8]| {
                ui.pairing.with_value(|p| p.keypair.as_ref().is_some_and(|pair| pair.is_mine(key)))
            };
            let made = match b64::decode(&their_key) {
                Some(key) if own(&key) => Err("That’s your own key"),
                Some(key) => kem::encapsulate(&key).map_err(|_| "That key doesn’t look right"),
                None => Err("That key doesn’t look right"),
            };
            match made {
                Ok((reply, secret)) => {
                    ui.pairing.update_value(|p| p.answer = Some(Rc::new(Answer { their_key, reply, secret })));
                    d.enable_reply();
                }
                Err(why) => d.disable_reply(why),
            }
        },
        Duration::from_millis(400),
    );
    d.state.update_value(|s| s.timer = timer.ok());
}

/// Opens the pairing dialog. In edit mode the existing keypair is reused (so the
/// user's public key stays the same); only the other person's key / reply are
/// pre-filled for correction.
pub fn start_pairing(ui: Ui, is_edit: bool) {
    let d = ui.pair;
    if !is_edit {
        ui.pairing.update_value(|p| {
            p.answer = None;
            p.keypair = Some(Rc::new(KeyPair::generate()));
        });
    }

    d.title.set(if is_edit { "Edit the session" } else { "Pair a session" });
    d.start_label.set(if is_edit { "Update" } else { "Pair" });
    d.error.clear();
    d.pk_shown.set(false);
    d.ct_shown.set(false);

    // Pre-fill the two codes when editing.
    let (stored_pk, stored_ct, answer) =
        ui.pairing.with_value(|p| (p.other_pk.clone(), p.other_ct.clone(), p.answer.clone()));
    let prefill = |node: NodeRef<html::Input>, stored: &Option<String>| {
        if let Some(input) = node.get_untracked() {
            input.set_value(if is_edit { stored.as_deref().unwrap_or("") } else { "" });
        }
    };
    prefill(d.other_pk, &stored_pk);
    prefill(d.other_ct, &stored_ct);

    d.state.update_value(|s| {
        s.open = true;
        s.is_edit = is_edit;
        s.orig_answer = answer;
    });
    // Whatever is in the "their key" box now, say where my reply to it stands:
    // nothing to reply to yet, the reply it already has, or one to be made.
    on_their_key(ui);

    if let Some(dialog) = d.dialog.get_untracked() {
        dom::show_modal(&dialog);
    }
    if let Some(copy) = d.copy_pk.get_untracked() {
        let _ = copy.focus();
    }
}

fn on_start(ui: Ui) {
    let d = ui.pair;
    if !d.state.with_value(|s| s.open) {
        return;
    }
    let (their_key, their_reply) = (d.field(d.other_pk), d.field(d.other_ct));
    if their_key.is_empty() || their_reply.is_empty() {
        d.error.show(
            face::WAVE,
            "Missing a code",
            "Paste both the other person’s key and their reply.",
        );
        let empty = if their_key.is_empty() { d.other_pk } else { d.other_ct };
        if let Some(input) = empty.get_untracked() {
            let _ = input.focus();
        }
        return;
    }
    // My answer has to be the one made for the key that is in the box now.
    let (keypair, answer) = ui.pairing.with_value(|p| {
        (p.keypair.clone(), p.answer.clone().filter(|a| a.their_key == their_key))
    });
    let (Some(keypair), Some(answer)) = (keypair, answer) else {
        // Nothing is on its way if the key was refused; the line by "Copy my
        // reply" already says why.
        if d.ct_status.get_untracked() == GENERATING {
            d.error.show(
                face::WAVE,
                "Not ready yet",
                "Your reply is still generating. Give it a moment after pasting their key.",
            );
        } else {
            d.error.show(
                face::HUH,
                "Their key isn’t right",
                "Paste the key the other person sent you, the whole of it.",
            );
            if let Some(input) = d.other_pk.get_untracked() {
                let _ = input.focus();
            }
        }
        return;
    };

    // The session is worked out while the dialog is still open, so that a wrong
    // code can be put right there and then. Closing on a mistake would throw
    // the whole exchange away: the next attempt starts with a new key of mine,
    // and the one already sent is no use to anybody.
    let paired = b64::decode(&their_key)
        .zip(b64::decode(&their_reply))
        .ok_or(PairError::BadCode)
        .and_then(|(key, reply)| keypair.pair(&key, &answer.reply, &answer.secret, &reply));
    let combined = match paired {
        Ok(combined) => combined,
        Err(wrong) => {
            let (field, title, body) = match wrong {
                PairError::OwnKey => (
                    d.other_pk,
                    "That’s your own key",
                    "Paste the key the other person sent you, not the one you copied here.",
                ),
                PairError::OwnReply => (
                    d.other_ct,
                    "That’s your own reply",
                    "Paste the reply the other person sent you, not the one you copied here.",
                ),
                PairError::KeyForReply => (
                    d.other_ct,
                    "That’s a key, not a reply",
                    "Their reply is the second code they sent you. Paste that one here.",
                ),
                PairError::BadCode => (
                    d.other_ct,
                    "That reply isn’t right",
                    "Paste the reply the other person sent you, the whole of it.",
                ),
            };
            d.error.show(face::HUH, title, body);
            if let Some(input) = field.get_untracked() {
                let _ = input.focus();
            }
            return;
        }
    };

    d.cleanup();
    spawn_local(async move {
        d.close().await;
        ui.pairing.update_value(|p| {
            p.other_pk = Some(their_key);
            p.other_ct = Some(their_reply);
        });
        // Updated with the very codes it was paired with, a session is the same
        // session, and keeps what it is remembered as. Any other outcome is a new
        // key and so a new session: whatever the last one was remembered as stays
        // behind with it.
        let unchanged = ui.session.with_untracked(|s| s.as_ref().is_some_and(|s| s.secret == combined));
        if !unchanged {
            ui.session.set(Some(Session::new(&combined, None)));
        }
        let name = kem::session_name(&combined);
        ui.error.clear();
        dom::after_render(move || {
            if let Some(el) = ui.session_name_el.get_untracked() {
                dom::focus(&el);
            }
        });
        ui.announce(format!("Paired as {name}. Compare the check number to be sure of it."));
    });
}

fn on_cancel(ui: Ui) {
    let d = ui.pair;
    if !d.state.with_value(|s| s.open) {
        return;
    }
    // Put back the answer the session was paired with, in case another key was
    // tried in the box and answered; a fresh attempt leaves nothing behind.
    let (is_edit, answer) = d.state.with_value(|s| (s.is_edit, s.orig_answer.clone()));
    ui.pairing.update_value(|p| p.answer = if is_edit { answer } else { None });
    d.cleanup();
    spawn_local(d.close());
}

fn end_session(ui: Ui) {
    ui.pairing.set_value(Pairing::default());
    ui.session.set(None);
    ui.error.clear();
    dom::after_render(move || {
        if let Some(btn) = ui.start_pairing_btn.get_untracked() {
            let _ = btn.focus();
        }
    });
    ui.announce("Session ended");
}

/// The session card (when there is a session), and otherwise the remembered
/// sessions to open and the invitation to pair.
#[component]
pub fn SessionPanel(ui: Ui) -> impl IntoView {
    let d = ui.pair;
    let paired = move || ui.session.with(|s| s.is_some());
    // Opened from the remembered ones: there is no keypair to copy from or edit.
    let restored = move || ui.session.with(|s| s.as_ref().is_some_and(|s| s.restored));
    let session_text = move |f: fn(&Session) -> &str| {
        ui.session.with(|s| s.as_ref().map(f).unwrap_or("—").to_owned())
    };

    // The check number, a group at a time for the eye, and a digit at a time for
    // a screen reader (which would otherwise read out eight five-figure sums).
    let check = move || ui.session.with(|s| s.as_ref().map(|s| s.check.clone()).unwrap_or_default());
    let check_groups = move || {
        check()
            .split(' ')
            .map(|group| view! { <span aria-hidden="true">{group.to_owned()}</span> })
            .collect_view()
    };
    let check_spoken = move || {
        check()
            .split(' ')
            .map(|group| group.chars().map(String::from).collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>()
            .join(", ")
    };

    let copy_key = move |ev: ev::MouseEvent| {
        if let Some(key) = public_key(ui) {
            ui.copy(key, d.key_flash, dom::button_of(&ev), ui.error);
        }
    };
    let copy_reply = move |ev: ev::MouseEvent| {
        if let Some(reply) = my_reply(ui) {
            ui.copy(reply, d.reply_flash, dom::button_of(&ev), ui.error);
        }
    };

    view! {
        <div id="session-on" class="session-card" hidden=move || !paired()>
            <div class="session-head">
                <div class="session-name">
                    <span class="dot" aria-hidden="true"></span>
                    <span id="session-name" class="mono" tabindex="-1" node_ref=ui.session_name_el>
                        {move || session_text(Session::title)}
                    </span>
                </div>
                <span id="session-meta" class="meta mono">
                    {move || if restored() { session_text(|s| &s.name) } else { "paired".to_owned() }}
                    <span class="label-long">{move || if restored() { " · remembered" } else { " · ML-KEM-1024" }}</span>
                </span>
            </div>
            <p id="session-note" class="note">
                {move || {
                    if restored() {
                        "Opened from this device. Ending the session locks it away again."
                    } else {
                        "You should both see this exact name. If theirs is different, edit the session. The name only catches a slip; the check number is the proof."
                    }
                }}
            </p>
            // Swapping codes can't tell whose codes arrived. Two people who read
            // each other the same number are in the same session, with nobody between.
            <div id="session-check" class="session-check">
                <p class="eyebrow">"Check number"</p>
                <p id="session-check-number" class="session-check-number mono">
                    <span class="sr-only">{check_spoken}</span>
                    {check_groups}
                </p>
                <p class="note">
                    "Compare all of it on a call or in person, not in the chat the codes went through. The same number on both sides means nobody is in the middle."
                </p>
            </div>
            <div class="session-actions">
                <button type="button" id="copy-key" class="btn btn-soft" aria-label="Copy my key" hidden=restored on:click=copy_key>
                    {icon("copy")}
                    <span class="label-long">{d.key_flash.or("Copy my key")}</span>
                    <span class="label-short">{d.key_flash.or("Key")}</span>
                </button>
                <button type="button" id="copy-reply" class="btn btn-soft" aria-label="Copy my reply" hidden=restored on:click=copy_reply>
                    {icon("copy")}
                    <span class="label-long">{d.reply_flash.or("Copy my reply")}</span>
                    <span class="label-short">{d.reply_flash.or("Reply")}</span>
                </button>
                <button type="button" id="edit-session" class="btn btn-soft" hidden=restored on:click=move |_| start_pairing(ui, true)>
                    <span>"Edit"</span>
                </button>
                <button type="button" id="end-session" class="btn btn-danger" aria-label="End session" on:click=move |_| end_session(ui)>
                    <span class="label-long">"End session"</span>
                    <span class="label-short">"End"</span>
                </button>
            </div>
            <remember::KeepRow ui/>
        </div>
        <remember::KeptList ui/>
        <div id="session-off" class="session-empty" hidden=paired>
            <p class="session-empty-title">"Not paired yet"</p>
            <p class="note">
                "Swap two codes with the other person once. After that you can both lock and unlock messages, no password needed."
            </p>
            <button
                type="button"
                id="start-pairing"
                class="btn btn-accent-outline"
                node_ref=ui.start_pairing_btn
                on:click=move |_| start_pairing(ui, false)
            >
                <span>"Start pairing"</span>
                {icon("arrow")}
            </button>
        </div>
    }
}

#[component]
pub fn PairDialog(ui: Ui) -> impl IntoView {
    let d = ui.pair;

    let copy_my_key = move |ev: ev::MouseEvent| {
        if let Some(key) = public_key(ui) {
            ui.copy(key, d.pk_flash, dom::button_of(&ev), d.error);
        }
    };
    let copy_my_reply = move |ev: ev::MouseEvent| {
        if let Some(reply) = my_reply(ui) {
            ui.copy(reply, d.ct_flash, dom::button_of(&ev), d.error);
        }
    };
    let on_esc = move |e: ev::Event| {
        e.prevent_default(); // we close with our own animation
        on_cancel(ui);
    };

    view! {
        <dialog
            id="pair-dialog"
            class="dialog"
            aria-labelledby="pair-title"
            aria-describedby="pair-desc"
            node_ref=d.dialog
            on:cancel=on_esc
        >
            <div class="dialog-head">
                <h2 id="pair-title">{move || d.title.get()}</h2>
                <p id="pair-desc" class="note">
                    "Trade two codes with the other person. Nothing leaves this browser except what you copy."
                </p>
            </div>
            <ol class="steps">
                <li class="step">
                    <span class="step-n mono" aria-hidden="true">"1"</span>
                    <div class="step-body">
                        <p class="step-label">"Send them your key"</p>
                        <div class="step-row">
                            <button type="button" id="copy-pk-link" class="btn btn-soft" node_ref=d.copy_pk on:click=copy_my_key>
                                {icon("copy")}
                                <span>{d.pk_flash.or("Copy my key")}</span>
                            </button>
                        </div>
                    </div>
                </li>
                <li class="step">
                    <span class="step-n mono" aria-hidden="true">"2"</span>
                    <div class="step-body">
                        <label for="session-other-pk" class="step-label">"Paste their key"</label>
                        {secret_input("session-other-pk", "Their key", "their key", d.pk_shown, d.other_pk, move || on_their_key(ui))}
                    </div>
                </li>
                <li class="step">
                    <span class="step-n mono" aria-hidden="true">"3"</span>
                    <div class="step-body">
                        <p class="step-label">"Send them your reply"</p>
                        <div class="step-row">
                            <button
                                type="button"
                                id="copy-ct-link"
                                class="btn btn-soft"
                                disabled=move || !d.reply_ready.get()
                                on:click=copy_my_reply
                            >
                                {icon("copy")}
                                <span>{d.ct_flash.or("Copy my reply")}</span>
                            </button>
                            <span id="ct-status" class="meta mono" aria-live="polite">{move || d.ct_status.get()}</span>
                        </div>
                    </div>
                </li>
                <li class="step">
                    <span class="step-n mono" aria-hidden="true">"4"</span>
                    <div class="step-body">
                        <label for="session-other-ct" class="step-label">"Paste their reply"</label>
                        {secret_input("session-other-ct", "Their reply", "their reply", d.ct_shown, d.other_ct, move || d.error.clear())}
                    </div>
                </li>
            </ol>
            {d.error.view("dialog-error")}
            <div class="dialog-actions">
                <button type="button" id="session-cancel-btn" class="btn btn-secondary" on:click=move |_| on_cancel(ui)>
                    <span>"Cancel"</span>
                </button>
                <button type="button" id="session-start-btn" class="btn btn-primary" on:click=move |_| on_start(ui)>
                    <span>{move || d.start_label.get()}</span>
                </button>
            </div>
        </dialog>
    }
}
