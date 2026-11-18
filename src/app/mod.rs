//! The page itself, as Leptos components.
//!
//! The markup mirrors what the hand-written HTML was (same ids and classes), so
//! the stylesheet applies unchanged.

mod compose;
mod dom;
mod pair;
mod remember;
mod result;

use std::{rc::Rc, time::Duration};

use js_sys::JsString;
use leptos::{html, prelude::*, task::spawn_local};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{Blob, Element, HtmlElement};

use crate::{
    b64,
    kem::{self, KeyPair, Secret},
};

/// huH is 5.25× the size of what goes in, and the whole run happens in memory on
/// the page's own thread, so files are capped where it still feels snappy.
pub const MAX_FILE_BYTES: f64 = 10.0 * 1024.0 * 1024.0;
pub const MAX_HUH_BYTES: f64 = 6.0 * MAX_FILE_BYTES;

pub mod face {
    pub const ALARM: &str = "ヾ( ■_■)ノ♪";
    pub const WAVE: &str = "( ・_・)ノ";
    pub const HUH: &str = "( ・◇・)?";
    pub const GRUMP: &str = "（︶^︶）";
    pub const DEAD: &str = "(×_×)";
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Password,
    Session,
}

/// The step a lock or an unlock is on. Locking compresses, then encrypts;
/// unlocking decrypts, then decompresses (when there is anything to decompress).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Compress,
    Encrypt,
    Decrypt,
    Decompress,
}

impl Step {
    pub fn locking(self) -> bool {
        matches!(self, Step::Compress | Step::Encrypt)
    }

    /// Whether this is the second step of its direction.
    pub fn second(self) -> bool {
        matches!(self, Step::Encrypt | Step::Decompress)
    }

    pub fn label(self) -> &'static str {
        match self {
            Step::Compress => "Compressing…",
            Step::Encrypt => "Encrypting…",
            Step::Decrypt => "Decrypting…",
            Step::Decompress => "Decompressing…",
        }
    }
}

/// Each step stays up at least this long, so it can be read even when the work
/// itself takes a few milliseconds. Skipped when the reader asked for less motion.
pub const STEP_MIN_MS: f64 = 350.0;

/// A file waiting in the compose box. `huh` holds its text when it is a .huh to
/// unlock; otherwise it is a file to lock.
#[derive(Clone)]
pub struct Attached {
    pub file: web_sys::File,
    pub name: String,
    pub mime: String,
    pub meta: String,
    pub huh: Option<Rc<str>>,
    pub thumb: Option<String>,
}

/// The session in use: the AES key (base64 of the combined secrets) and what it goes by.
#[derive(Clone)]
pub struct Session {
    pub key: String,
    /// The name both people see (amber-arc-07), worked out from the key.
    pub name: String,
    /// The nickname it is remembered under on this device, if it is.
    pub nick: Option<String>,
    /// Opened from the remembered ones rather than paired just now, so there is
    /// no keypair behind it: nothing to copy, nothing to edit.
    pub restored: bool,
}

impl Session {
    /// The session a secret makes: just paired, or just opened again as `nick`.
    pub fn new(secret: &Secret, nick: Option<String>) -> Self {
        Self {
            key: b64::encode(secret),
            name: kem::session_name(secret),
            restored: nick.is_some(),
            nick,
        }
    }

    /// What it is called on screen. Fresh from pairing that is the name both
    /// people compare; opened again later, it is the nickname it was given.
    pub fn title(&self) -> &str {
        match &self.nick {
            Some(nick) if self.restored => nick,
            _ => &self.name,
        }
    }

    /// The secret behind the key.
    pub fn secret(&self) -> Option<Secret> {
        b64::decode(&self.key)?.try_into().ok()
    }
}

/// Both parties run the same flow: each generates a keypair, exchanges public
/// keys, encapsulates the other's key to get (my reply, my secret), then
/// decapsulates the other's reply. The final key is the XOR of the two secrets.
#[derive(Default)]
pub struct Pairing {
    pub keypair: Option<Rc<KeyPair>>,
    /// What I send back after encapsulating their key.
    pub my_reply: Option<Rc<Vec<u8>>>,
    pub my_secret: Option<Secret>,
    /// Remembered for "edit session".
    pub other_pk: Option<String>,
    pub other_ct: Option<String>,
}

/// What the result bubble is showing.
#[derive(Clone)]
pub struct Shown {
    pub locked: bool,
    /// "with a password" / "in amber-arc-07"
    pub via: String,
    pub meta: String,
    pub body: Body,
}

#[derive(Clone)]
pub enum Body {
    /// huH or plain text in the bubble.
    Text { text: JsString, huh: bool },
    /// A file locked into huH: a .huh to save (or copy).
    HuhFile { text: JsString, blob: Option<Blob>, name: String, size: String, sample: String },
    /// A huH that unlocked into a file.
    File { blob: Option<Blob>, name: String, size: String, preview: Option<String>, is_image: bool },
}

/// An inline error: a face, a title and a line of help.
#[derive(Clone, Copy)]
pub struct ErrorBox {
    root: NodeRef<html::Div>,
    face: NodeRef<html::Span>,
    title: NodeRef<html::P>,
    body: NodeRef<html::P>,
}

impl ErrorBox {
    pub fn new() -> Self {
        Self { root: NodeRef::new(), face: NodeRef::new(), title: NodeRef::new(), body: NodeRef::new() }
    }

    pub fn show(self, face: &'static str, title: &str, body: &str) {
        let (Some(root), Some(face_el), Some(title_el), Some(body_el)) = (
            self.root.get_untracked(),
            self.face.get_untracked(),
            self.title.get_untracked(),
            self.body.get_untracked(),
        ) else {
            return;
        };
        // Empty the box first, so a repeated, identical error is still a fresh
        // insertion that role="alert" announces.
        face_el.set_text_content(Some(""));
        title_el.set_text_content(Some(""));
        body_el.set_text_content(Some(""));
        root.set_hidden(false);
        dom::restart_animation(&root, "shake");
        let (title, body) = (title.to_owned(), body.to_owned());
        request_animation_frame(move || {
            face_el.set_text_content(Some(face));
            title_el.set_text_content(Some(&title));
            body_el.set_text_content(Some(&body));
        });
    }

    pub fn clear(self) {
        if let Some(root) = self.root.get_untracked() {
            root.set_hidden(true);
        }
    }

    pub fn view(self, id: &'static str) -> impl IntoView {
        view! {
            <div id=id class="error" role="alert" hidden node_ref=self.root>
                <span class="face mono" aria-hidden="true" node_ref=self.face></span>
                <div class="error-text">
                    <p class="error-title" node_ref=self.title></p>
                    <p class="error-body" node_ref=self.body></p>
                </div>
            </div>
        }
    }
}

/// A label that is briefly swapped for something like "Copied!".
#[derive(Clone, Copy)]
pub struct Flash {
    text: RwSignal<Option<&'static str>>,
    timer: StoredValue<Option<TimeoutHandle>>,
}

impl Flash {
    pub fn new() -> Self {
        Self { text: RwSignal::new(None), timer: StoredValue::new(None) }
    }

    pub fn show(self, text: &'static str) {
        self.stop_timer();
        self.text.set(Some(text));
        let handle = set_timeout_with_handle(
            move || {
                self.timer.set_value(None);
                self.text.set(None);
            },
            Duration::from_millis(1600),
        );
        self.timer.set_value(handle.ok());
    }

    pub fn cancel(self) {
        self.stop_timer();
        self.text.set(None);
    }

    fn stop_timer(self) {
        if let Some(handle) = self.timer.get_value() {
            handle.clear();
        }
        self.timer.set_value(None);
    }

    /// The flash while it lasts, else the label's normal text.
    pub fn or(self, normal: &'static str) -> impl Fn() -> &'static str + Copy {
        move || self.text.get().unwrap_or(normal)
    }
}

/// Everything the components share. All handles, so it copies freely.
#[derive(Clone, Copy)]
pub struct Ui {
    pub mode: RwSignal<Mode>,
    /// Whether the typed message is itself a huH (so Decrypt becomes the main button).
    pub text_is_huh: RwSignal<bool>,
    pub attached: RwSignal<Option<Attached>, LocalStorage>,
    /// The step in progress, while something is being locked or unlocked.
    pub job: RwSignal<Option<Step>>,
    pub session: RwSignal<Option<Session>>,
    pub pairing: StoredValue<Pairing, LocalStorage>,
    pub result: RwSignal<Option<Shown>, LocalStorage>,
    /// Object URL behind the unlocked-image preview.
    pub preview_url: StoredValue<Option<String>>,
    pub error: ErrorBox,
    pub copy_flash: Flash,
    pub save_flash: Flash,
    pub text_in: NodeRef<html::Textarea>,
    pub pass_in: NodeRef<html::Input>,
    pub session_name_el: NodeRef<html::Span>,
    pub start_pairing_btn: NodeRef<html::Button>,
    pub sr: NodeRef<html::Div>,
    pub pair: pair::PairUi,
    pub keep: remember::KeepUi,
}

impl Ui {
    fn new() -> Self {
        Self {
            mode: RwSignal::new(Mode::Password),
            text_is_huh: RwSignal::new(false),
            attached: RwSignal::new_local(None),
            job: RwSignal::new(None),
            session: RwSignal::new(None),
            pairing: StoredValue::new_local(Pairing::default()),
            result: RwSignal::new_local(None),
            preview_url: StoredValue::new(None),
            error: ErrorBox::new(),
            copy_flash: Flash::new(),
            save_flash: Flash::new(),
            text_in: NodeRef::new(),
            pass_in: NodeRef::new(),
            session_name_el: NodeRef::new(),
            start_pairing_btn: NodeRef::new(),
            sr: NodeRef::new(),
            pair: pair::PairUi::new(),
            keep: remember::KeepUi::new(),
        }
    }

    /// Polite status messages for screen readers (copied, paired, ended).
    pub fn announce(self, text: impl Into<String>) {
        let Some(sr) = self.sr.get_untracked() else { return };
        let text = text.into();
        sr.set_text_content(Some(""));
        request_animation_frame(move || sr.set_text_content(Some(&text)));
    }

    /// Moves on to a step: shows it, says it, and waits for it to be on screen
    /// before the (blocking) work starts. Returns when the step began.
    pub async fn enter(self, step: Step) -> f64 {
        let began = js_sys::Date::now();
        self.job.set(Some(step));
        self.announce(step.label());
        dom::next_paint().await;
        began
    }

    /// Keeps a step that began at `began` up for its minimum time.
    pub async fn linger(self, began: f64) {
        let left = STEP_MIN_MS - (js_sys::Date::now() - began);
        if left > 0.0 && !dom::reduced_motion() {
            dom::sleep(left as u64).await;
        }
    }

    /// "with a password" or "in <session name>", for the mode in use right now.
    pub fn via(self) -> String {
        match (self.mode.get_untracked(), self.session.get_untracked()) {
            (Mode::Session, Some(session)) => format!("in {}", session.title()),
            _ => "with a password".to_owned(),
        }
    }

    /// Copies text, flashing `flash` on success and explaining in `on_fail` otherwise.
    pub fn copy(self, text: JsString, flash: Flash, btn: Option<HtmlElement>, on_fail: ErrorBox) {
        spawn_local(async move {
            let mut ok = dom::write_clipboard(&text).await;
            if !ok {
                let host: Option<Element> = match self.pair.dialog.get_untracked() {
                    Some(dialog) if dialog.open() => Some(dialog.into()),
                    _ => document().body().map(Into::into),
                };
                ok = host.is_some_and(|host| dom::legacy_copy(&text, &host));
                if let Some(btn) = &btn {
                    dom::focus(btn);
                }
            }
            if ok {
                flash.show("Copied!");
                self.announce("Copied to clipboard");
            } else {
                on_fail.show(
                    face::WAVE,
                    "Hold up!",
                    "Copying might not work in this browser setting. Select the text and copy it by hand.",
                );
            }
        });
    }
}

pub fn fmt_size(n: f64) -> String {
    let n = n.max(0.0) as u64;
    // Tenths, rounded half up, as toFixed did.
    let tenths = |unit: u64| (n * 10 + unit / 2) / unit;
    if n < 1024 {
        format!("{n} B")
    } else if n < 1024 * 1024 {
        if n < 10 * 1024 {
            let t = tenths(1024);
            format!("{}.{} KB", t / 10, t % 10)
        } else {
            format!("{} KB", (n + 512) / 1024)
        }
    } else {
        let t = tenths(1024 * 1024);
        format!("{}.{} MB", t / 10, t % 10)
    }
}

/// The inside of an icon's `<svg>`: a reference into the sprite. It is set as
/// markup so the `<use>` is the only child, exactly as the stylesheet expects.
pub fn sprite(name: &str) -> String {
    format!(r##"<use href="#i-{name}"></use>"##)
}

/// An icon from the sprite.
pub fn icon(name: &'static str) -> impl IntoView {
    view! { <svg aria-hidden="true" class="icon" inner_html=sprite(name)></svg> }
}

/// Icon sprite: referenced with `<use href="#i-…">`.
const SPRITE: &str = concat!(
    "<defs>",
    r##"<symbol id="i-key" viewBox="0 0 24 24"><circle cx="8" cy="15" r="4"></circle><path d="M10.8 12.2 20 3"></path><path d="M16.5 6.5l3 3"></path><path d="M14 9l2 2"></path></symbol>"##,
    r##"<symbol id="i-session" viewBox="0 0 24 24"><path d="M4 8h14"></path><path d="M15 5l3 3-3 3"></path><path d="M20 16H6"></path><path d="M9 13l-3 3 3 3"></path></symbol>"##,
    r##"<symbol id="i-eye-off" viewBox="0 0 24 24"><path d="M3 10c2.4 3 5.4 4.5 9 4.5s6.6-1.5 9-4.5"></path><path d="M6.2 13.2 4.6 15.6"></path><path d="M12 14.5v2.8"></path><path d="M17.8 13.2l1.6 2.4"></path></symbol>"##,
    r##"<symbol id="i-eye" viewBox="0 0 24 24"><path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12z"></path><circle cx="12" cy="12" r="3"></circle></symbol>"##,
    r##"<symbol id="i-copy" viewBox="0 0 24 24"><rect x="9" y="9" width="11" height="11" rx="2.5"></rect><path d="M5 15V6.5A2.5 2.5 0 0 1 7.5 4H15"></path></symbol>"##,
    r##"<symbol id="i-lock" viewBox="0 0 24 24"><rect x="4.5" y="10.5" width="15" height="10" rx="2.5"></rect><path d="M8 10.5V7.5a4 4 0 0 1 8 0v3"></path></symbol>"##,
    r##"<symbol id="i-unlock" viewBox="0 0 24 24"><rect x="4.5" y="10.5" width="15" height="10" rx="2.5"></rect><path d="M8 10.5V7.5a4 4 0 0 1 7.6-1.8"></path></symbol>"##,
    r##"<symbol id="i-arrow" viewBox="0 0 24 24"><path d="M5 12h14"></path><path d="M13 6l6 6-6 6"></path></symbol>"##,
    r##"<symbol id="i-clip" viewBox="0 0 24 24"><path d="M20.3 11.6l-7.6 7.6a5 5 0 0 1-7.1-7.1l8.3-8.3a3.2 3.2 0 0 1 4.5 4.5l-8.3 8.3a1.5 1.5 0 0 1-2.1-2.1l7.6-7.6"></path></symbol>"##,
    r##"<symbol id="i-download" viewBox="0 0 24 24"><path d="M12 4v11"></path><path d="M7.5 10.5 12 15l4.5-4.5"></path><path d="M5 19.5h14"></path></symbol>"##,
    r##"<symbol id="i-x" viewBox="0 0 24 24"><path d="M6 6l12 12"></path><path d="M18 6 6 18"></path></symbol>"##,
    r##"<symbol id="i-file" viewBox="0 0 24 24"><path d="M7 3.5h7l5 5V20a1.5 1.5 0 0 1-1.5 1.5H7A1.5 1.5 0 0 1 5.5 20V5A1.5 1.5 0 0 1 7 3.5z"></path><path d="M14 3.5V9h5"></path></symbol>"##,
    r##"<symbol id="i-bookmark" viewBox="0 0 24 24"><path d="M7.5 3.5h9A1.5 1.5 0 0 1 18 5v15.5l-6-4-6 4V5a1.5 1.5 0 0 1 1.5-1.5z"></path></symbol>"##,
    "</defs>",
);

/// The constellation behind the page, drawn by particles.js (loaded by index.html).
const PARTICLES: &str = r##"{
  "particles": {
    "number": { "value": 50, "density": { "enable": true, "value_area": 1000 } },
    "color": { "value": "#ffffff" },
    "shape": {
      "type": "circle",
      "stroke": { "width": 2, "color": "#ffffff" },
      "polygon": { "nb_sides": 5 },
      "image": { "src": "img/github.svg", "width": 100, "height": 100 }
    },
    "opacity": { "value": 1, "random": false, "anim": { "enable": false, "speed": 1, "opacity_min": 0.9, "sync": false } },
    "size": { "value": 3, "random": true, "anim": { "enable": false, "speed": 40, "size_min": 0.1, "sync": false } },
    "line_linked": { "enable": true, "distance": 150, "color": "#ffffff", "opacity": 1, "width": 1 },
    "move": {
      "enable": true, "speed": 6, "direction": "none", "random": false, "straight": false,
      "out_mode": "out", "bounce": false,
      "attract": { "enable": false, "rotateX": 600, "rotateY": 1200 }
    }
  },
  "interactivity": {
    "detect_on": "canvas",
    "events": {
      "onhover": { "enable": false, "mode": "repulse" },
      "onclick": { "enable": false, "mode": "push" },
      "resize": true
    },
    "modes": {
      "grab": { "distance": 400, "line_linked": { "opacity": 1 } },
      "bubble": { "distance": 400, "size": 40, "duration": 2, "opacity": 8, "speed": 3 },
      "repulse": { "distance": 200, "duration": 0.4 },
      "push": { "particles_nb": 4 },
      "remove": { "particles_nb": 2 }
    }
  },
  "retina_detect": true
}"##;

/// Starts the particle background. With "reduce motion" switched on, the
/// constellation is drawn once and holds still.
fn start_particles() {
    use js_sys::{Function, Reflect, JSON};

    let win = window();
    let Some(particles_js) = Reflect::get(&win, &JsValue::from_str("particlesJS"))
        .ok()
        .and_then(|f| f.dyn_into::<Function>().ok())
    else {
        return; // the library didn't load; the page works without its backdrop
    };
    let Ok(config) = JSON::parse(PARTICLES) else { return };
    let reduce_motion = dom::reduced_motion();
    let movement = Reflect::get(&config, &JsValue::from_str("particles"))
        .and_then(|p| Reflect::get(&p, &JsValue::from_str("move")));
    if let Ok(movement) = movement {
        let _ = Reflect::set(&movement, &JsValue::from_str("enable"), &JsValue::from_bool(!reduce_motion));
    }
    let _ = particles_js.call2(&JsValue::NULL, &JsValue::from_str("particles-js"), &config);
}

#[component]
fn App() -> impl IntoView {
    let ui = Ui::new();
    remember::start(ui);

    // Click bounce on every button.
    dom::on_document("click", |ev| {
        let Some(btn) = dom::button_of(&ev) else { return };
        if !btn.has_attribute("disabled") {
            dom::restart_animation(&btn, "bounce-anim");
        }
    });

    // A file dropped anywhere outside the compose card must not navigate away from the app.
    for event in ["dragover", "drop"] {
        dom::on_document(event, |ev| {
            if ev.dyn_ref::<web_sys::DragEvent>().is_some_and(compose::drag_has_files) {
                ev.prevent_default();
            }
        });
    }

    // Once the page is on screen: the backdrop, and the caret in the message box.
    Effect::new(move |_| {
        start_particles();
        if let Some(text) = ui.text_in.get_untracked() {
            dom::focus(&text);
        }
    });

    view! {
        <svg aria-hidden="true" width="0" height="0" style="position:absolute" inner_html=SPRITE></svg>
        <div id="particles-js" aria-hidden="true"></div>

        <div class="page">
            <header class="topbar">
                <div class="brand">
                    <img src="./img/logo.png" alt="Huh" width="56" height="56" class="brand-logo"/>
                    <h1 class="eyebrow brand-name">"encrypter"</h1>
                </div>
                <p class="tagline mono">"ML-KEM-1024 + AES-256 · runs entirely in your browser"</p>
            </header>

            <main class="layout">
                <compose::Compose ui/>
                <result::ResultPane ui/>
            </main>
        </div>

        <pair::PairDialog ui/>
        <remember::KeepDialog ui/>

        <div id="sr-status" class="sr-only" role="status" aria-live="polite" node_ref=ui.sr></div>
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
