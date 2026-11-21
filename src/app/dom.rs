//! Small browser helpers the page leans on.

use std::time::Duration;

use js_sys::{Array, Function, JsString, Promise, Reflect, Uint8Array};
use leptos::{
    prelude::*,
    task::{spawn_local, tick},
};
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, BlobPropertyBag, Element, FocusOptions, HtmlDialogElement, HtmlElement, Url};

/// Focuses without scrolling the page to the element.
pub fn focus(el: &HtmlElement) {
    let options = FocusOptions::new();
    options.set_prevent_scroll(true);
    let _ = el.focus_with_options(&options);
}

/// Runs once the page has caught up with the state that was just set (so an
/// element that state un-hides can be focused, say).
pub fn after_render(f: impl FnOnce() + 'static) {
    spawn_local(async move {
        tick().await;
        tick().await;
        f();
    });
}

/// Resolves after `ms` milliseconds.
pub async fn sleep(ms: u64) {
    let promise = Promise::new(&mut |resolve, _| {
        set_timeout(
            move || {
                let _ = resolve.call0(&JsValue::NULL);
            },
            Duration::from_millis(ms),
        );
    });
    let _ = JsFuture::from(promise).await;
}

/// Resolves once the browser has painted, so a label set just before a long,
/// blocking run is actually on screen while it runs. A tab that isn't being
/// shown gets no frames, so a timer stands in rather than leave the run hanging.
pub async fn next_paint() {
    let promise = Promise::new(&mut |resolve, _| {
        let painted = resolve.clone();
        request_animation_frame(move || {
            set_timeout(
                move || {
                    let _ = painted.call0(&JsValue::NULL);
                },
                Duration::ZERO,
            );
        });
        set_timeout(
            move || {
                let _ = resolve.call0(&JsValue::NULL);
            },
            Duration::from_millis(150),
        );
    });
    let _ = JsFuture::from(promise).await;
}

/// Whether the reader has asked their system for less motion.
pub fn reduced_motion() -> bool {
    window()
        .match_media("(prefers-reduced-motion: reduce)")
        .ok()
        .flatten()
        .is_some_and(|m| m.matches())
}

/// Restarts a CSS animation that is driven by a class.
pub fn restart_animation(el: &Element, class: &str) {
    let list = el.class_list();
    let _ = list.remove_1(class);
    if let Some(html) = el.dyn_ref::<HtmlElement>() {
        let _ = html.offset_width(); // force a reflow so the animation starts over
    }
    let _ = list.add_1(class);
}

/// The button a click landed in (events are delegated, so `currentTarget` isn't it).
pub fn button_of(ev: &web_sys::Event) -> Option<HtmlElement> {
    let target: Element = ev.target()?.dyn_into().ok()?;
    target.closest("button").ok()??.dyn_into().ok()
}

/// A listener on `document` that lives as long as the page.
pub fn on_document(event: &str, f: impl FnMut(web_sys::Event) + 'static) {
    let closure = Closure::<dyn FnMut(web_sys::Event)>::new(f);
    let _ = document().add_event_listener_with_callback(event, closure.as_ref().unchecked_ref());
    closure.forget();
}

/// A listener on `window` that lives as long as the page.
pub fn on_window(event: &str, f: impl FnMut(web_sys::Event) + 'static) {
    let closure = Closure::<dyn FnMut(web_sys::Event)>::new(f);
    let _ = window().add_event_listener_with_callback(event, closure.as_ref().unchecked_ref());
    closure.forget();
}

/// Opens a `<dialog>` over the page, which stays put behind it.
pub fn show_modal(dialog: &HtmlDialogElement) {
    let _ = dialog.show_modal();
    if let Some(body) = document().body() {
        let _ = body.class_list().add_1("modal-open");
    }
}

/// Lets the page behind a dialog move again. For every way a dialog comes to
/// close: by this page, or by the browser on its own account (a second Escape
/// closes a dialog whatever the page said about the first).
pub fn modal_closed() {
    if let Some(body) = document().body() {
        let _ = body.class_list().remove_1("modal-open");
    }
}

/// Closes a dialog, letting its closing animation play first.
pub async fn close_modal(dialog: HtmlDialogElement) {
    if !dialog.open() {
        modal_closed();
        return;
    }
    let _ = dialog.class_list().add_1("closing");
    sleep(180).await;
    let _ = dialog.class_list().remove_1("closing");
    dialog.close();
    modal_closed();
}

/// This browser's storage for the page, where it lets the page have any (a
/// private window or a strict setting may not).
fn storage() -> Option<web_sys::Storage> {
    window().local_storage().ok().flatten()
}

/// What is stored under `key`, if anything is.
pub fn stored(key: &str) -> Option<String> {
    storage()?.get_item(key).ok().flatten()
}

/// Stores `value` under `key`. Says whether the browser took it.
pub fn store(key: &str, value: &str) -> bool {
    storage().is_some_and(|s| s.set_item(key, value).is_ok())
}

/// PBKDF2-HMAC-SHA256 by the browser's own crypto: 32 bytes of key, worked out
/// natively and off the page's thread. `None` where the page isn't offered
/// `crypto.subtle` (browsers keep it to https and localhost) or it won't do it.
pub async fn pbkdf2(pass: &str, salt: &[u8], rounds: u32) -> Option<[u8; 32]> {
    let crypto = window().crypto().ok()?;
    let subtle = Reflect::get(&crypto, &JsValue::from_str("subtle")).ok()?;
    if subtle.is_undefined() || subtle.is_null() {
        return None;
    }
    let subtle: web_sys::SubtleCrypto = subtle.unchecked_into();

    let usages = Array::of1(&JsValue::from_str("deriveBits"));
    let imported = subtle
        .import_key_with_str("raw", &Uint8Array::from(pass.as_bytes()), "PBKDF2", false, &usages)
        .ok()?;
    let base: web_sys::CryptoKey = JsFuture::from(imported).await.ok()?.unchecked_into();

    let how = js_sys::Object::new();
    for (name, value) in [
        ("name", JsValue::from_str("PBKDF2")),
        ("hash", JsValue::from_str("SHA-256")),
        ("salt", Uint8Array::from(salt).into()),
        ("iterations", JsValue::from_f64(f64::from(rounds))),
    ] {
        Reflect::set(&how, &JsValue::from_str(name), &value).ok()?;
    }
    let bits = JsFuture::from(subtle.derive_bits_with_object(&how, &base, 256).ok()?).await.ok()?;
    Uint8Array::new(&bits).to_vec().try_into().ok()
}

/// Stretches a password or passphrase into a key: PBKDF2-HMAC-SHA256, which is
/// slow on purpose. The browser's own crypto does the sum where the page has it;
/// otherwise this program does the very same one itself, which holds the page
/// still while it runs.
pub async fn stretch(pass: &str, salt: &[u8], rounds: u32) -> [u8; 32] {
    match pbkdf2(pass, salt, rounds).await {
        Some(key) => key,
        None => {
            next_paint().await;
            crate::kdf::pbkdf2(pass, salt, rounds)
        }
    }
}

/// Text as the same bytes however the keyboard put its accents together (an "é"
/// can arrive as one character or as two).
pub fn nfc(text: &str) -> String {
    JsString::from(text).normalize("NFC").as_string().unwrap_or_else(|| text.to_owned())
}

/// An element's `value`, as the JavaScript string it is (no conversion, no loss).
pub fn raw_value(el: &JsValue) -> JsString {
    Reflect::get(el, &JsValue::from_str("value"))
        .ok()
        .and_then(|v| v.dyn_into().ok())
        .unwrap_or_else(|| JsString::from(""))
}

pub fn set_raw_value(el: &JsValue, value: &JsString) {
    let _ = Reflect::set(el, &JsValue::from_str("value"), value);
}

/// What a thrown JavaScript value says, if it says anything.
pub fn js_message(err: &JsValue) -> Option<String> {
    let msg: String = err.dyn_ref::<js_sys::Error>()?.message().into();
    (!msg.is_empty()).then_some(msg)
}

pub async fn file_bytes(file: &web_sys::File) -> Result<Vec<u8>, JsValue> {
    let buffer = JsFuture::from(file.array_buffer()).await?;
    Ok(Uint8Array::new(&buffer).to_vec())
}

pub async fn file_text(file: &web_sys::File) -> Result<String, JsValue> {
    JsFuture::from(file.text()).await?.as_string().ok_or(JsValue::NULL)
}

fn blob_options(mime: &str) -> BlobPropertyBag {
    let bag = BlobPropertyBag::new();
    bag.set_type(mime);
    bag
}

pub fn blob_from_bytes(bytes: &[u8], mime: &str) -> Option<Blob> {
    let parts = Array::of1(&Uint8Array::from(bytes));
    Blob::new_with_u8_array_sequence_and_options(&parts, &blob_options(mime)).ok()
}

pub fn blob_from_text(text: &JsString, mime: &str) -> Option<Blob> {
    Blob::new_with_str_sequence_and_options(&Array::of1(text), &blob_options(mime)).ok()
}

pub fn object_url(blob: &Blob) -> Option<String> {
    Url::create_object_url_with_blob(blob).ok()
}

pub fn revoke_url(url: &str) {
    let _ = Url::revoke_object_url(url);
}

/// Saves a blob through a throwaway download link.
pub fn download_blob(blob: &Blob, name: &str) {
    let Some(url) = object_url(blob) else { return };
    let doc = document();
    let Ok(a) = doc.create_element("a") else { return };
    let _ = a.set_attribute("href", &url);
    let _ = a.set_attribute("download", name);
    let _ = a.set_attribute("hidden", "");
    if let Some(body) = doc.body() {
        let _ = body.append_child(&a);
    }
    if let Some(a) = a.dyn_ref::<HtmlElement>() {
        a.click();
    }
    a.remove();
    set_timeout(move || revoke_url(&url), Duration::from_secs(10));
}

/// The async clipboard, where the browser offers it (secure contexts only).
pub async fn write_clipboard(text: &JsString) -> bool {
    let win = window();
    if !win.is_secure_context() {
        return false;
    }
    let Ok(clipboard) = Reflect::get(&win.navigator(), &JsValue::from_str("clipboard")) else {
        return false;
    };
    if clipboard.is_undefined() || clipboard.is_null() {
        return false;
    }
    let write = Reflect::get(&clipboard, &JsValue::from_str("writeText"))
        .ok()
        .and_then(|f| f.dyn_into::<Function>().ok());
    let Some(write) = write else { return false };
    match write.call1(&clipboard, text).map(|p| p.dyn_into::<Promise>()) {
        Ok(Ok(promise)) => JsFuture::from(promise).await.is_ok(),
        _ => false,
    }
}

/// Legacy path (no async clipboard, e.g. plain http): select the text in a
/// throwaway textarea and `execCommand('copy')`. The textarea is created inside
/// `host`, which is the open dialog when there is one, since everything outside a
/// modal `<dialog>` is inert.
pub fn legacy_copy(text: &JsString, host: &Element) -> bool {
    let doc = document();
    let Ok(sink) = doc.create_element("textarea") else { return false };
    sink.set_class_name("sink");
    let _ = sink.set_attribute("aria-hidden", "true");
    let _ = sink.set_attribute("tabindex", "-1");
    set_raw_value(&sink, text);
    let _ = host.append_child(&sink);

    let copied = (|| {
        let area = sink.dyn_ref::<web_sys::HtmlTextAreaElement>()?;
        focus(area);
        if doc.active_element().as_ref() != Some(&sink) {
            return None;
        }
        area.select();
        let _ = area.set_selection_range(0, text.length());
        doc.dyn_ref::<web_sys::HtmlDocument>()?.exec_command("copy").ok()
    })()
    .unwrap_or(false);

    sink.remove();
    copied
}
