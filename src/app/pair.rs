//! Session pairing: the panel in the compose card and the dialog that trades the two codes.

use std::{rc::Rc, time::Duration};

use js_sys::JsString;
use leptos::{ev, html, prelude::*, task::spawn_local};

use super::{compose::secret_input, dom, face, icon, ErrorBox, Flash, Pairing, Session, Ui};
use crate::{
    b64, huh,
    kem::{self, KeyPair, Secret},
};

/// What the open dialog needs to remember.
#[derive(Default)]
struct DialogState {
    open: bool,
    is_edit: bool,
    /// The reply and secret from before the dialog opened, to put back on cancel or failure.
    orig_reply: Option<Rc<Vec<u8>>>,
    orig_secret: Option<Secret>,
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
        let Some(dialog) = self.dialog.get_untracked() else { return };
        if !dialog.open() {
            return;
        }
        let _ = dialog.class_list().add_1("closing");
        dom::sleep(180).await;
        let _ = dialog.class_list().remove_1("closing");
        dialog.close();
        if let Some(body) = document().body() {
            let _ = body.class_list().remove_1("modal-open");
        }
    }
}

fn public_key(ui: Ui) -> Option<JsString> {
    ui.pairing
        .with_value(|p| p.keypair.as_ref().map(|k| JsString::from(b64::encode(k.public()))))
}

fn my_reply(ui: Ui) -> Option<JsString> {
    ui.pairing
        .with_value(|p| p.my_reply.as_ref().map(|r| JsString::from(b64::encode(r))))
}

/// Makes the reply whenever the other person's key changes. Any reply made for an
/// earlier key is void the moment the key changes, so Pair's "Not ready yet" guard
/// holds until the new one is ready.
fn on_their_key(ui: Ui) {
    let d = ui.pair;
    d.error.clear();
    ui.pairing.update_value(|p| {
        p.my_reply = None;
        p.my_secret = None;
    });
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
    d.disable_reply("Generating…");
    let timer = set_timeout_with_handle(
        move || {
            if d.state.with_value(|s| s.generation != mine) {
                return; // superseded meanwhile
            }
            let made = b64::decode(&their_key).and_then(|pk| kem::encapsulate(&pk).ok());
            match made {
                Some((reply, secret)) => {
                    ui.pairing.update_value(|p| {
                        p.my_reply = Some(Rc::new(reply));
                        p.my_secret = Some(secret);
                    });
                    d.enable_reply();
                }
                None => d.disable_reply("That key doesn’t look right"),
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
            p.my_reply = None;
            p.my_secret = None;
            p.keypair = Some(Rc::new(KeyPair::generate()));
        });
    }

    d.title.set(if is_edit { "Edit the session" } else { "Pair a session" });
    d.start_label.set(if is_edit { "Update" } else { "Pair" });
    d.error.clear();
    d.pk_shown.set(false);
    d.ct_shown.set(false);

    // Pre-fill the two codes when editing.
    let (stored_pk, stored_ct, reply, secret) = ui.pairing.with_value(|p| {
        (p.other_pk.clone(), p.other_ct.clone(), p.my_reply.clone(), p.my_secret)
    });
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
        s.orig_reply = reply.clone();
        s.orig_secret = secret;
        s.generation += 1;
    });

    // Restore the saved reply if editing with one; otherwise it needs their key.
    if is_edit && reply.is_some() {
        d.enable_reply();
    } else {
        d.disable_reply("Needs their key first");
    }
    // If editing with a stored key but no reply yet, make it now.
    if is_edit && stored_pk.is_some_and(|pk| !pk.is_empty()) && reply.is_none() {
        on_their_key(ui);
    }

    if let Some(dialog) = d.dialog.get_untracked() {
        let _ = dialog.show_modal();
    }
    if let Some(body) = document().body() {
        let _ = body.class_list().add_1("modal-open");
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
    let (keypair, secret) = ui.pairing.with_value(|p| (p.keypair.clone(), p.my_secret));
    let ready = ui.pairing.with_value(|p| p.my_reply.is_some());
    let (Some(keypair), Some(secret), true) = (keypair, secret, ready) else {
        d.error.show(
            face::WAVE,
            "Not ready yet",
            "Your reply is still generating. Give it a moment after pasting their key.",
        );
        return;
    };

    d.cleanup();
    spawn_local(async move {
        d.close().await;
        // XOR is commutative: both parties arrive at the same combined key.
        let combined = b64::decode(&their_reply)
            .and_then(|reply| keypair.decapsulate(&reply).ok())
            .map(|theirs| kem::combine(&secret, &theirs));
        match combined {
            Some(combined) => {
                let name = kem::session_name(&combined);
                ui.pairing.update_value(|p| {
                    p.other_pk = Some(their_key);
                    p.other_ct = Some(their_reply);
                });
                ui.session.set(Some(Session { key: b64::encode(&combined), name: name.clone() }));
                ui.error.clear();
                dom::after_render(move || {
                    if let Some(el) = ui.session_name_el.get_untracked() {
                        dom::focus(&el);
                    }
                });
                ui.announce(format!("Paired as {name}"));
            }
            None => {
                // Leave whatever was paired before exactly as it was.
                let (reply, secret) =
                    d.state.with_value(|s| (s.orig_reply.clone(), s.orig_secret));
                ui.pairing.update_value(|p| {
                    p.my_reply = reply;
                    p.my_secret = secret;
                });
                ui.error.show(face::DEAD, "Pairing failed", "Check the codes and try again.");
            }
        }
    });
}

fn on_cancel(ui: Ui) {
    let d = ui.pair;
    if !d.state.with_value(|s| s.open) {
        return;
    }
    // Restore the reply if the user changed the key field then cancelled; a fresh
    // attempt leaves nothing behind.
    let (is_edit, reply, secret) =
        d.state.with_value(|s| (s.is_edit, s.orig_reply.clone(), s.orig_secret));
    ui.pairing.update_value(|p| {
        p.my_reply = if is_edit { reply } else { None };
        p.my_secret = if is_edit { secret } else { None };
    });
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

/// The session card (when paired) and the invitation to pair (when not).
#[component]
pub fn SessionPanel(ui: Ui) -> impl IntoView {
    let d = ui.pair;
    let paired = move || ui.session.with(|s| s.is_some());

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
                        {move || ui.session.with(|s| s.as_ref().map(|s| s.name.clone()).unwrap_or_else(|| "—".to_owned()))}
                    </span>
                </div>
                <span class="meta mono">"paired"<span class="label-long">" · ML-KEM-1024"</span></span>
            </div>
            <p class="note">"You should both see this exact name. If theirs is different, edit the session."</p>
            <div class="session-actions">
                <button type="button" id="copy-key" class="btn btn-soft" aria-label="Copy my key" on:click=copy_key>
                    {icon("copy")}
                    <span class="label-long">{d.key_flash.or("Copy my key")}</span>
                    <span class="label-short">{d.key_flash.or("Key")}</span>
                </button>
                <button type="button" id="copy-reply" class="btn btn-soft" aria-label="Copy my reply" on:click=copy_reply>
                    {icon("copy")}
                    <span class="label-long">{d.reply_flash.or("Copy my reply")}</span>
                    <span class="label-short">{d.reply_flash.or("Reply")}</span>
                </button>
                <button type="button" id="edit-session" class="btn btn-soft" on:click=move |_| start_pairing(ui, true)>
                    <span>"Edit"</span>
                </button>
                <button type="button" id="end-session" class="btn btn-danger" aria-label="End session" on:click=move |_| end_session(ui)>
                    <span class="label-long">"End session"</span>
                    <span class="label-short">"End"</span>
                </button>
            </div>
        </div>
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
