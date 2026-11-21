//! The result: a speech bubble holding huH or text, or a card for a file.

use js_sys::JsString;
use leptos::{ev, html, prelude::*};
use web_sys::{HtmlDivElement, HtmlTextAreaElement};

use super::{dom, fmt_size, icon, sprite, Body, Shown, Ui};

/// Lets go of what the last result held on to.
fn let_go(ui: Ui) {
    if let Some(url) = ui.preview_url.get_value() {
        dom::revoke_url(&url);
    }
    ui.preview_url.set_value(None);
    ui.copy_flash.cancel();
    ui.save_flash.cancel();
}

/// Shared plumbing for every new result.
fn begin(ui: Ui, shown: Shown) {
    let_go(ui);
    ui.preview_url.set_value(match &shown.body {
        Body::File { preview, .. } => preview.clone(),
        _ => None,
    });
    ui.error.clear();
    ui.result.set(Some(shown));
}

/// Takes the last result off the page. For when something was asked for and
/// didn't come off: what was there before would look like its outcome (a huH
/// that didn't open under the last thing that did; a new message under the last
/// huH, ready to copy).
pub fn clear(ui: Ui) {
    let_go(ui);
    ui.result.set(None);
}

/// What an old huH was locked with. It decides what there is to say about it.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Old {
    Password,
    Session,
}

impl Old {
    /// What comes with the result of an old huH. The old envelope had no check
    /// on it, so nothing would have noticed a change made on the way; and its
    /// password took one round of MD5 to try, so every old huH still around is
    /// a quick way to find the password it was locked with.
    fn note(self) -> &'static str {
        match self {
            Old::Password => {
                "This huH was locked the old way. Nothing checked it for changes on its way, and old huHs make their password quick to guess: use a new password for anything new."
            }
            Old::Session => "This huH was locked the old way. Nothing checked it for changes on its way.",
        }
    }
}

/// How a result came about.
pub struct How {
    /// What it was locked or unlocked with: "with a password", "in amber-arc-07".
    /// Taken when the button was pressed (`Ui::via`), since the mode or the
    /// session can be changed while the work is still going on.
    pub via: String,
    /// Set when it came out of a huH in the old envelope.
    pub old: Option<Old>,
}

impl How {
    fn label(&self) -> String {
        match self.old {
            Some(_) => format!("{} · old huH, unchecked", self.via),
            None => self.via.clone(),
        }
    }

    fn note(&self) -> Option<&'static str> {
        self.old.map(Old::note)
    }
}

/// huH or unlocked text, in the bubble.
pub fn show_text(ui: Ui, value: &str, locked: bool, how: How) {
    let text = JsString::from(value);
    let chars = text.length();
    let meta = if locked { format!("{chars} chars of huh") } else { format!("{chars} chars") };
    // The steps were read out as they went; say how it ended too.
    ui.announce(match (locked, how.note()) {
        (true, _) => "Locked and ready to send".to_owned(),
        (false, None) => "Unlocked".to_owned(),
        (false, Some(note)) => format!("Unlocked. {note}"),
    });
    begin(ui, Shown { locked, via: how.label(), meta, note: how.note(), body: Body::Text { text, huh: locked } });
}

/// A file locked into huH: offer it as a .huh to save (or copy, for small ones).
pub fn show_huh_file(ui: Ui, huh: &str, name: String, via: String) {
    let size = format!("{} of huH", fmt_size(huh.len() as f64));
    let sample = match huh.get(..120) {
        Some(head) if huh.len() > 120 => format!("{head}…"),
        _ => huh.to_owned(),
    };
    let text = JsString::from(huh);
    let blob = dom::blob_from_text(&text, "text/plain;charset=utf-8");
    ui.announce(format!("Locked {name}"));
    begin(
        ui,
        Shown {
            locked: true,
            via,
            meta: size.clone(),
            note: None,
            body: Body::HuhFile { text, blob, name, size, sample },
        },
    );
}

/// The picture types the page shows a preview of, and hands back as what they
/// say they are. Everything else is handed back as plain bytes to save.
const PICTURES: [&str; 6] = ["image/png", "image/jpeg", "image/gif", "image/webp", "image/avif", "image/bmp"];

/// A huH that unlocked into a file: rebuild it and offer it back.
///
/// The file's type is whatever the person who locked it said it was, so it is
/// not taken at its word. Only plain pictures are previewed and given their own
/// type. Anything else (a web page, an SVG, which is a document that can carry
/// script) becomes a download of bytes that the browser has no reason to open
/// as a page of this site. The type is still shown, as text.
pub fn show_unlocked_file(ui: Ui, name: &str, mime: &str, bytes: &[u8], how: How) {
    let name = if name.is_empty() { "file".to_owned() } else { name.replace(['/', '\\'], "_") };
    let mime = if mime.is_empty() { "application/octet-stream" } else { mime };
    let is_image = PICTURES.contains(&mime);
    let size = format!("{} · {mime}", fmt_size(bytes.len() as f64));
    let blob = dom::blob_from_bytes(bytes, if is_image { mime } else { "application/octet-stream" });
    let preview = if is_image { blob.as_ref().and_then(dom::object_url) } else { None };
    ui.announce(match how.note() {
        Some(note) => format!("Unlocked {name}. {note}"),
        None => format!("Unlocked {name}"),
    });
    begin(
        ui,
        Shown {
            locked: false,
            via: how.label(),
            meta: size.clone(),
            note: how.note(),
            body: Body::File { blob, name, size, preview, is_image },
        },
    );
}

/// The bubble's fade lifts once the text is scrolled to the end.
fn update_fade(bubble: &HtmlDivElement, result: &HtmlTextAreaElement) {
    let at_end = result.scroll_top() + result.client_height() >= result.scroll_height() - 2;
    let _ = bubble.class_list().toggle_with_force("at-end", at_end);
}

/// A long result is capped (`--result-max` in the stylesheet) and scrolls inside
/// the bubble, under a fade. Returns whether it is capped.
fn resize(bubble: &HtmlDivElement, result: &HtmlTextAreaElement) -> bool {
    let classes = bubble.class_list();
    let _ = classes.remove_1("clamped");
    // (Named in full: Leptos has a `style` of its own on elements.)
    let style = web_sys::HtmlElement::style(result);
    let _ = style.set_property("height", "1px");
    let _ = style.set_property("height", &format!("{}px", result.scroll_height()));
    let clamped = result.scroll_height() > result.client_height() + 1;
    let _ = classes.toggle_with_force("clamped", clamped);
    // Width a classic scrollbar takes, so the fade stops short of it.
    let scrollbar = result.offset_width() - result.client_width();
    let _ = web_sys::HtmlElement::style(bubble).set_property("--sb", &format!("{scrollbar}px"));
    update_fade(bubble, result);
    clamped
}

#[component]
pub fn ResultPane(ui: Ui) -> impl IntoView {
    let out = NodeRef::<html::Div>::new();
    let bubble = NodeRef::<html::Div>::new();
    let result = NodeRef::<html::Textarea>::new();
    let clamped = RwSignal::new(false);

    let with_body = move |f: fn(&Body) -> bool| ui.result.with(|r| r.as_ref().is_some_and(|s| f(&s.body)));
    let is_text = move || with_body(|b| matches!(b, Body::Text { .. }));
    let is_file = move || with_body(|b| matches!(b, Body::File { .. }));
    let is_huh_file = move || with_body(|b| matches!(b, Body::HuhFile { .. }));
    let preview = move || {
        ui.result.with(|r| match r.as_ref().map(|s| &s.body) {
            Some(Body::File { preview, .. }) => preview.clone(),
            _ => None,
        })
    };
    let file_text = move |f: fn(&Body) -> Option<&String>| {
        ui.result.with(|r| r.as_ref().and_then(|s| f(&s.body)).cloned().unwrap_or_default())
    };
    let note = move || ui.result.with(|r| r.as_ref().and_then(|shown| shown.note));
    // While something is being locked or unlocked, the steps take the result's place.
    let working = move || ui.job.get().is_some();
    let locking = move || ui.job.get().is_none_or(|step| step.locking());
    let second = move || ui.job.get().is_some_and(|step| step.second());
    let shown_text = move |f: fn(&Shown) -> String| {
        if working() {
            return String::new();
        }
        ui.result.with(|r| r.as_ref().map(f).unwrap_or_default())
    };

    // Every new result pops in; text is put in the bubble and the bubble sized to it.
    // This waits for a frame so the page has caught up with the state first.
    Effect::new(move |_| {
        let Some(shown) = ui.result.get() else { return };
        request_animation_frame(move || {
            let (Some(out), Some(bubble), Some(result)) =
                (out.get_untracked(), bubble.get_untracked(), result.get_untracked())
            else {
                return;
            };
            let _ = bubble.class_list().remove_2("clamped", "at-end");
            dom::restart_animation(&out, "pop");
            match &shown.body {
                Body::Text { text, .. } => {
                    dom::set_raw_value(&result, text);
                    result.set_scroll_top(0);
                    clamped.set(resize(&bubble, &result));
                }
                _ => clamped.set(false),
            }
        });
    });

    let _ = window_event_listener(ev::resize, move |_| {
        // Only a text result that is on screen can be measured.
        let text_shown = ui.result.with_untracked(|r| matches!(r.as_ref().map(|s| &s.body), Some(Body::Text { .. })));
        if !text_shown || ui.job.get_untracked().is_some() {
            return;
        }
        if let (Some(bubble), Some(result)) = (bubble.get_untracked(), result.get_untracked()) {
            clamped.set(resize(&bubble, &result));
        }
    });

    let on_scroll = move |_| {
        if let (Some(bubble), Some(result)) = (bubble.get_untracked(), result.get_untracked()) {
            update_fade(&bubble, &result);
        }
    };

    let on_copy = move |ev: ev::MouseEvent| {
        let text = ui.result.with_untracked(|r| match r.as_ref().map(|s| &s.body) {
            Some(Body::Text { text, .. }) | Some(Body::HuhFile { text, .. }) => Some(text.clone()),
            _ => None,
        });
        if let Some(text) = text {
            ui.copy(text, ui.copy_flash, dom::button_of(&ev), ui.error);
        }
    };

    let on_save = move |_| {
        let file = ui.result.with_untracked(|r| match r.as_ref().map(|s| &s.body) {
            Some(Body::HuhFile { blob: Some(blob), name, .. }) | Some(Body::File { blob: Some(blob), name, .. }) => {
                Some((blob.clone(), name.clone()))
            }
            _ => None,
        });
        if let Some((blob, name)) = file {
            dom::download_blob(&blob, &name);
            ui.save_flash.show("Saved!");
            ui.announce(format!("Saved {name}"));
        }
    };

    let save_label = move || {
        let normal = ui.result.with(|r| match r.as_ref().map(|s| &s.body) {
            Some(Body::File { is_image: true, .. }) => "Save image",
            Some(Body::File { .. }) => "Save file",
            _ => "Save .huh",
        });
        ui.save_flash.or(normal)()
    };
    let copy_label = move || {
        let normal = ui.result.with(|r| match r.as_ref().map(|s| &s.body) {
            Some(Body::Text { huh: false, .. }) => "Copy text",
            _ => "Copy huH",
        });
        ui.copy_flash.or(normal)()
    };

    view! {
        <section class="result" aria-labelledby="result-label">
            <div class="result-head">
                <h2 id="result-label" class="eyebrow">
                    {move || match ui.job.get() {
                        Some(step) if step.locking() => "Locking…",
                        Some(_) => "Unlocking…",
                        None => ui.result.with(|r| match r {
                            None => "Result",
                            Some(s) if s.locked => "Locked · ready to send",
                            Some(_) => "Unlocked",
                        }),
                    }}
                </h2>
                <span id="result-meta" class="meta mono">{move || shown_text(|s| s.meta.clone())}</span>
                <span id="result-meta-phone" class="meta mono">{move || shown_text(|s| s.via.clone())}</span>
            </div>

            // At work: the two steps, ticked off as they finish, under three reels
            // that shuffle h, u and H (backwards when unlocking).
            <div id="result-working" class="working" class:unlocking=move || !locking() hidden=move || !working()>
                <div class="reels mono" aria-hidden="true">
                    {(0..5)
                        .map(|_| {
                            view! {
                                <span class="reel">
                                    <span class="reel-strip">
                                        <span>"h"</span>
                                        <span>"u"</span>
                                        <span>"H"</span>
                                        <span>"h"</span>
                                    </span>
                                </span>
                            }
                        })
                        .collect_view()}
                </div>
                <ol class="working-steps">
                    <li class="working-step" class:active=move || !second() class:done=second>
                        <span class="tick" aria-hidden="true"></span>
                        <span>{move || if locking() { "Compressing" } else { "Decrypting" }}</span>
                    </li>
                    // Unlocking only gets a second step if there turns out to be something to decompress.
                    <li class="working-step" class:active=second hidden=move || !locking() && !second()>
                        <span class="tick" aria-hidden="true"></span>
                        <span>{move || if locking() { "Encrypting" } else { "Decompressing" }}</span>
                    </li>
                </ol>
            </div>

            <div id="result-out" hidden=move || working() || ui.result.with(|r| r.is_none()) node_ref=out>
                <div class="bubble-wrap">
                    <div class="bubble" node_ref=bubble>
                        <textarea
                            id="result"
                            rows="1"
                            readonly
                            aria-label="Result"
                            spellcheck="false"
                            class:huh=move || with_body(|b| matches!(b, Body::Text { huh: true, .. }))
                            hidden=move || is_file() || is_huh_file()
                            on:scroll=on_scroll
                            node_ref=result
                        ></textarea>
                        // File result: shown instead of the text.
                        <div id="file-card" class="file-card" hidden=move || !(is_file() || is_huh_file())>
                            <img
                                id="file-preview"
                                class="file-preview"
                                alt="Unlocked image"
                                hidden=move || preview().is_none()
                                src=preview
                            />
                            <div class="file-row">
                                <div id="file-glyph" class="file-glyph mono" aria-hidden="true" hidden=move || preview().is_some()>
                                    <span id="file-glyph-text" hidden=is_file>"huH"</span>
                                    <svg class="icon" id="file-glyph-icon" hidden=move || !is_file() inner_html=sprite("file")></svg>
                                </div>
                                <div class="file-info">
                                    <p id="file-name" class="file-name">
                                        {move || file_text(|b| match b {
                                            Body::HuhFile { name, .. } | Body::File { name, .. } => Some(name),
                                            _ => None,
                                        })}
                                    </p>
                                    <p id="file-size" class="file-size mono">
                                        {move || file_text(|b| match b {
                                            Body::HuhFile { size, .. } | Body::File { size, .. } => Some(size),
                                            _ => None,
                                        })}
                                    </p>
                                </div>
                            </div>
                            <p id="file-sample" class="file-sample mono" aria-hidden="true" hidden=is_file>
                                {move || file_text(|b| match b {
                                    Body::HuhFile { sample, .. } => Some(sample),
                                    _ => None,
                                })}
                            </p>
                        </div>
                    </div>
                    <svg aria-hidden="true" class="bubble-tail" width="46" height="34" viewBox="0 0 46 34">
                        <path d="M6 0H40C30 9 20 20 3 33C9 22 10 10 6 0Z"></path>
                    </svg>
                </div>
                <div class="result-actions">
                    <button type="button" id="save-btn" class="btn btn-light" hidden=move || !(is_file() || is_huh_file()) on:click=on_save>
                        {icon("download")}
                        <span id="save-label">{save_label}</span>
                    </button>
                    <button type="button" id="copy-btn" class="btn btn-light" hidden=is_file on:click=on_copy>
                        {icon("copy")}
                        <span id="copy-label">{copy_label}</span>
                    </button>
                    <span id="locked-with" class="meta mono">
                        {move || shown_text(|s| format!("{} {}", if s.locked { "locked" } else { "unlocked" }, s.via))}
                    </span>
                    <span id="result-more" class="meta mono" hidden=move || !(clamped.get() && is_text())>
                        "copy grabs all of it"
                    </span>
                </div>
                // What there is to know about an old huH, under what it unlocked to.
                <p id="result-note" class="note result-note" hidden=move || note().is_none()>
                    {move || note().unwrap_or_default()}
                </p>
            </div>

            <div id="result-empty" class="bubble-empty" hidden=move || working() || ui.result.with(|r| r.is_some())>
                <p class="bubble-empty-title">"Your huH lands here"</p>
                <p>
                    "Encrypt a message and it turns into a speech bubble you can paste anywhere. Attach a file or an image and it becomes a .huh you can send."
                </p>
            </div>
        </section>
    }
}
