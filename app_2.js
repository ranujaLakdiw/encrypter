// ─────────────────────────────────────────────────────────────
// Huh encrypter · app logic
// Crypto: AES (crypto-js) with either a shared password or an
// ML-KEM-1024 session secret. Output is "huH": base-3 text over
// the alphabet h / u / H, chunks joined with U.
// Files and images go through the same pipe: their bytes (plus a
// small name/type header) are AES-encrypted and packed into a .huh.
// ─────────────────────────────────────────────────────────────

const code = ['h', 'u', 'H'];

// ---------- PQC state ----------
// Both parties run the same flow: each generates a keypair, exchanges public
// keys, encapsulates the other's pk to get (myCt, myEncapSs), then decapsulates
// the other's CT (otherCt) with their sk to get myDecapSs.
// Final shared key = XOR(myEncapSs, myDecapSs) — commutative, so both sides agree.
var pkR       = null;  // my public key
var skR       = null;  // my private key (never leaves this device)
var myCt      = null;  // CT from encapsulating the other's pk (share with them)
var myEncapSs = null;  // shared secret I derived from encapping their pk
var ss        = null;  // final AES key (base64 of XOR'd secrets)
var session   = false;
var sessionName    = null;
var otherPkStored  = null; // remembered for "edit session"
var otherCtStored  = null;

const SESSION_WORDS_A = ['amber','brave','crisp','delta','ember','faint','ghost','haven','ivory','jewel','kite','lunar','mist','neon','onyx','pearl','quill','raven','slate','thorn'];
const SESSION_WORDS_B = ['arc','bay','cog','dune','echo','floe','gust','haze','isle','jade','knot','lore','maze','nova','orb','pike','reef','sage','tide','vale'];

// Derives a human-readable session name from the shared secret bytes.
// Both parties see the same name, confirming the exchange succeeded.
const deriveSessionName = (ssBytes) => {
    const a = SESSION_WORDS_A[ssBytes[0] % SESSION_WORDS_A.length];
    const b = SESSION_WORDS_B[ssBytes[1] % SESSION_WORDS_B.length];
    const n = ssBytes[2] % 100;
    return `${a}-${b}-${n < 10 ? '0' + n : n}`;
};

// ---------- DOM ----------
const $ = (id) => document.getElementById(id);

const textIn       = $('text');
const huhChip      = $('huh-chip');
const clearBtn     = $('clear');
const modeLabel    = $('mode-label');
const modePwBtn    = $('mode-password');
const modeSessBtn  = $('mode-session');
const panelPw      = $('panel-password');
const panelSess    = $('panel-session');
const sessionOn    = $('session-on');
const sessionOff   = $('session-off');
const sessionNameEl = $('session-name');
const passIn       = $('pass');
const copyKeyBtn   = $('copy-key');
const copyReplyBtn = $('copy-reply');
const editSessBtn  = $('edit-session');
const endSessBtn   = $('end-session');
const startPairBtn = $('start-pairing');
const encryptBtn   = $('encrypt-btn');
const decryptBtn   = $('decrypt-btn');
const errorBox     = $('error');

const resultLabel  = $('result-label');
const resultMeta   = $('result-meta');
const resultMetaPhone = $('result-meta-phone');
const resultOut    = $('result-out');
const resultEmpty  = $('result-empty');
const result       = $('result');
const copyBtn      = $('copy-btn');
const copyLabel    = $('copy-label');
const lockedWith   = $('locked-with');
const resultMore   = $('result-more');
const bubble       = result.closest('.bubble');
const saveBtn      = $('save-btn');
const saveLabel    = $('save-label');
const fileCard     = $('file-card');
const filePreview  = $('file-preview');
const fileGlyph    = $('file-glyph');
const fileGlyphText = $('file-glyph-text');
const fileGlyphIcon = $('file-glyph-icon');
const fileNameEl   = $('file-name');
const fileSizeEl   = $('file-size');
const fileSample   = $('file-sample');

const composeEl    = document.querySelector('.compose');
const attachBtn    = $('attach-btn');
const fileInput    = $('file-input');
const attachCard   = $('attach-card');
const attachThumb  = $('attach-thumb');
const attachGlyph  = $('attach-glyph');
const attachGlyphText = $('attach-glyph-text');
const attachGlyphIcon = $('attach-glyph-icon');
const attachName   = $('attach-name');
const attachMeta   = $('attach-meta');
const attachRemove = $('attach-remove');

const dialogEl     = $('pair-dialog');
const dialogTitle  = $('pair-title');
const dialogError  = $('dialog-error');
const copyPkLink   = $('copy-pk-link');
const copyCtLink   = $('copy-ct-link');
const ctStatus     = $('ct-status');
const otherPkIn    = $('session-other-pk');
const otherCtIn    = $('session-other-ct');
const startBtn     = $('session-start-btn');
const cancelBtn    = $('session-cancel-btn');
const srStatus     = $('sr-status');

// ---------- UI state ----------
var mode = 'password';   // 'password' | 'session'
var resultKind = null;   // 'huh' | 'plain' | 'huh-file' | 'file' | null
var resultText = null;   // what "Copy" copies (huH or plain text)
var resultBlob = null;   // what "Save" downloads (a .huh, or the unlocked file)
var resultName = null;   // its filename
var previewUrl = null;   // object URL behind the unlocked-image preview

// Attached file waiting in the compose box.
// kind 'file' → will be locked into a .huh; kind 'huh' → a .huh to unlock (text holds it).
var attached  = null;    // { file, kind, text } | null
var attachUrl = null;    // object URL behind the attachment thumbnail

// huH is 5.25× the size of what goes in, and the whole run happens in memory
// in plain JavaScript, so files are capped where it still feels snappy.
const MAX_FILE_BYTES = 10 * 1024 * 1024;
const MAX_HUH_BYTES  = 6 * MAX_FILE_BYTES;

const FACE = {
    alarm: 'ヾ( ■_■)ノ♪',
    wave:  '( ・_・)ノ',
    huh:   '( ・◇・)?',
    grump: '（︶^︶）',
    dead:  '(×_×)',
};

const isHuhText = (s) => {
    const t = s.trim();
    return t.length > 8 && /^[huHU]+$/.test(t);
};

// ---------- Errors (inline, replaces the old modal alert) ----------
const fillError = (box, face, title, body) => {
    const faceEl  = box.querySelector('.face');
    const titleEl = box.querySelector('.error-title');
    const bodyEl  = box.querySelector('.error-body');
    // Empty the box first, so a repeated, identical error is still a fresh
    // insertion that role="alert" announces.
    faceEl.textContent = '';
    titleEl.textContent = '';
    bodyEl.textContent = '';
    box.hidden = false;
    box.classList.remove('shake');
    void box.offsetWidth; // restart the animation each time
    box.classList.add('shake');
    requestAnimationFrame(() => {
        faceEl.textContent = face || FACE.alarm;
        titleEl.textContent = title;
        bodyEl.textContent = body || '';
    });
};

// Polite status messages for screen readers (copied, paired, ended).
const announce = (text) => {
    srStatus.textContent = '';
    requestAnimationFrame(() => { srStatus.textContent = text; });
};

const showError = (face, title, body) => fillError(errorBox, face, title, body);
const clearError = () => { errorBox.hidden = true; };

const showDialogError = (face, title, body) => fillError(dialogError, face, title, body);
const clearDialogError = () => { dialogError.hidden = true; };

// Kept for callers that still use the old (title, body) signature.
const alertMsg = (title, body) => showError(FACE.alarm, title, body);

// ---------- Compose state ----------
const refreshCompose = () => {
    const huh = attached ? attached.kind === 'huh' : isHuhText(textIn.value);
    huhChip.hidden = !huh;
    huhChip.textContent = attached ? 'huH file' : 'huH detected';
    textIn.hidden = !!attached;
    attachCard.hidden = !attached;
    textIn.classList.toggle('huh', !attached && huh);
    modeLabel.textContent = huh ? 'Decrypt with' : 'Encrypt with';
    encryptBtn.classList.toggle('btn-primary', !huh);
    encryptBtn.classList.toggle('btn-secondary', huh);
    decryptBtn.classList.toggle('btn-primary', huh);
    decryptBtn.classList.toggle('btn-secondary', !huh);
};

const refreshSessionPanel = () => {
    sessionOn.hidden = !session;
    sessionOff.hidden = session;
    sessionNameEl.textContent = session ? sessionName : '—';
};

const setMode = (m) => {
    mode = m;
    const pw = m === 'password';
    modePwBtn.setAttribute('aria-pressed', pw ? 'true' : 'false');
    modeSessBtn.setAttribute('aria-pressed', pw ? 'false' : 'true');
    panelPw.hidden = !pw;
    panelSess.hidden = pw;
    clearError();
    if (!pw) refreshSessionPanel();
};

// ---------- Result ----------
// A long result is capped (--result-max in the stylesheet) and scrolls inside
// the bubble, under a fade that lifts once it's scrolled to the end.
const updateResultFade = () => {
    const atEnd = result.scrollTop + result.clientHeight >= result.scrollHeight - 2;
    bubble.classList.toggle('at-end', atEnd);
};

const resizeResult = () => {
    bubble.classList.remove('clamped');
    result.style.height = '1px';
    result.style.height = result.scrollHeight + 'px';
    const clamped = result.scrollHeight > result.clientHeight + 1;
    bubble.classList.toggle('clamped', clamped);
    // Width a classic scrollbar takes, so the fade stops short of it.
    bubble.style.setProperty('--sb', (result.offsetWidth - result.clientWidth) + 'px');
    resultMore.hidden = !clamped;
    updateResultFade();
};

const fmtSize = (n) => {
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(n < 10 * 1024 ? 1 : 0)} KB`;
    return `${(n / (1024 * 1024)).toFixed(1)} MB`;
};

// Shared result plumbing: header line, "locked with…" meta, pop animation.
const beginResult = (locked, via, meta) => {
    resultOut.hidden = false;
    resultEmpty.hidden = true;
    if (previewUrl) { URL.revokeObjectURL(previewUrl); previewUrl = null; }
    resultText = resultBlob = resultName = null;

    resultLabel.textContent = locked ? 'Locked · ready to send' : 'Unlocked';
    resultMeta.textContent = meta;
    const viaText = via === 'session' ? `in ${sessionName}` : 'with a password';
    lockedWith.textContent = (locked ? 'locked ' : 'unlocked ') + viaText;
    resultMetaPhone.textContent = viaText;
    cancelFlash(copyBtn);
    cancelFlash(saveBtn);
    bubble.classList.remove('clamped', 'at-end');
    resultMore.hidden = true;

    clearError();
    resultOut.classList.remove('pop');
    void resultOut.offsetWidth;
    resultOut.classList.add('pop');
};

const showResult = (value, kind, via) => {
    const locked = kind === 'huh';
    beginResult(locked, via, locked ? `${value.length} chars of huh` : `${value.length} chars`);
    resultKind = kind;
    resultText = value;
    result.value = value;
    result.classList.toggle('huh', locked);
    result.hidden = false;
    fileCard.hidden = true;
    saveBtn.hidden = true;
    copyBtn.hidden = false;
    copyLabel.textContent = locked ? 'Copy huH' : 'Copy text';
    result.scrollTop = 0;
    resizeResult();
};

// Fills the file card that sits in the bubble instead of the text.
const fillFileCard = ({ name, size, glyph, preview }) => {
    result.hidden = true;
    fileCard.hidden = false;
    fileNameEl.textContent = name;
    fileSizeEl.textContent = size;
    fileGlyph.hidden = !!preview;
    fileGlyphText.hidden = glyph !== 'huH';
    fileGlyphIcon.toggleAttribute('hidden', glyph !== 'file');
    filePreview.hidden = !preview;
    if (preview) filePreview.src = preview;
    else filePreview.removeAttribute('src');
};

// A file locked into huH: offer it as a .huh to save (or copy, for small ones).
const showHuhFileResult = (huh, name, via) => {
    const size = `${fmtSize(huh.length)} of huH`;
    beginResult(true, via, size);
    resultKind = 'huh-file';
    resultText = huh;
    resultBlob = new Blob([huh], { type: 'text/plain;charset=utf-8' });
    resultName = name;
    fillFileCard({ name, size, glyph: 'huH', preview: null });
    fileSample.hidden = false;
    fileSample.textContent = huh.slice(0, 120) + (huh.length > 120 ? '…' : '');
    saveBtn.hidden = false;
    saveLabel.textContent = 'Save .huh';
    copyBtn.hidden = false;
    copyLabel.textContent = 'Copy huH';
    announce(`Locked ${name}`);
};

// A huH that unlocked into a file: rebuild it and offer it back.
const showUnlockedFileResult = ({ name, type, bytes }, via) => {
    const safeName = (name || 'file').replace(/[\/\\]/g, '_');
    const mime = type || 'application/octet-stream';
    const isImage = /^image\//.test(mime);
    const size = `${fmtSize(bytes.length)} · ${mime}`;
    beginResult(false, via, size);
    resultKind = 'file';
    resultBlob = new Blob([bytes], { type: mime });
    resultName = safeName;
    if (isImage) previewUrl = URL.createObjectURL(resultBlob);
    fillFileCard({ name: safeName, size, glyph: 'file', preview: previewUrl });
    fileSample.hidden = true;
    saveBtn.hidden = false;
    saveLabel.textContent = isImage ? 'Save image' : 'Save file';
    copyBtn.hidden = true;
    announce(`Unlocked ${safeName}`);
};

const downloadBlob = (blob, name) => {
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = name;
    a.hidden = true;
    document.body.appendChild(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 10000);
};

// ---------- Clipboard ----------
// Temporarily swaps a button's label (every <span> inside it, so the
// desktop and phone variants of a label both change).
const flashLabel = (btn, text, ms = 1600) => {
    const spans = btn.querySelectorAll('span');
    const targets = spans.length ? Array.from(spans) : [btn];
    if (btn._flashTimer) {
        clearTimeout(btn._flashTimer);
        targets.forEach((el, i) => { el.textContent = btn._flashOriginal[i]; });
    }
    btn._flashOriginal = targets.map((el) => el.textContent);
    targets.forEach((el) => { el.textContent = text; });
    btn._flashTimer = setTimeout(() => {
        targets.forEach((el, i) => { el.textContent = btn._flashOriginal[i]; });
        btn._flashTimer = null;
    }, ms);
};

const cancelFlash = (btn) => {
    if (btn._flashTimer) {
        clearTimeout(btn._flashTimer);
        btn._flashTimer = null;
    }
};

// Legacy path (no async clipboard, e.g. plain http): select the text in a
// throwaway textarea and execCommand('copy'). The textarea is created inside
// the open dialog, since everything outside a modal <dialog> is inert.
const legacyCopy = (txt) => {
    const host = dialogEl.open ? dialogEl : document.body;
    const sink = document.createElement('textarea');
    sink.className = 'sink';
    sink.setAttribute('aria-hidden', 'true');
    sink.tabIndex = -1;
    sink.value = txt;
    host.appendChild(sink);
    try {
        sink.focus({ preventScroll: true });
        if (document.activeElement !== sink) return false;
        sink.select();
        sink.setSelectionRange(0, txt.length);
        return document.execCommand('copy');
    } finally {
        sink.remove();
    }
};

const copyText = async (txt, btn, onFail) => {
    const done = () => {
        if (btn) flashLabel(btn, 'Copied!');
        announce('Copied to clipboard');
    };
    if (window.isSecureContext && navigator.clipboard) {
        try {
            await navigator.clipboard.writeText(txt);
            done();
            return;
        } catch (_) { /* fall through to the legacy path */ }
    }
    let ok = false;
    try {
        ok = legacyCopy(txt);
    } catch (err) {
        console.error('Unable to copy to clipboard', err);
    }
    if (btn) btn.focus({ preventScroll: true });
    if (ok) { done(); return; }
    (onFail || showError)(FACE.wave, 'Hold up!',
        'Copying might not work in this browser setting. Select the text and copy it by hand.');
};

// ---------- huH codec ----------
const getIndex = (str, list) => list.findIndex((e) => str == e);

const getPower = (num) => {
    let count = 0;
    while (num >= 0) {
        if (num < 3 ** count)
            return count - 1;
        count++;
    }
};

// Base-3 digits, most significant first, no leading zeros (so to(0) is '').
// The digit is floored as a number: parseInt on the ratio would read a tiny
// remainder such as 2e-7 through its exponential string form and return 2.
const to = (i) => {
    let p = getPower(i);
    let out = '';
    while (p >= 0) {
        const step = 3 ** p;
        const x = Math.floor(i / step);
        out += code[x];
        i -= x * step;
        p--;
    }
    return out;
};

const from = (i) => {
    let num = 0;
    const li = i.split('');
    let p = li.length - 1;
    for (const e of li) {
        const index = getIndex(e, code);
        if (index < 0) {
            return -1;
        }
        num += index * (3 ** p);
        p--;
    }
    return num;
};

const compressToHuH = (base64_str) => {
    const binary_string = window.atob(base64_str);
    const arr = [];
    let i = 0;
    while (i < binary_string.length) {
        if (i + 3 <= binary_string.length) {
            arr.push(((binary_string.charCodeAt(i) << 16) | (binary_string.charCodeAt(i + 1) << 8) | binary_string.charCodeAt(i + 2)) + 65792);
            i += 3;
        } else if (i + 2 <= binary_string.length) {
            arr.push(((binary_string.charCodeAt(i) << 8) | binary_string.charCodeAt(i + 1)) + 256);
            i += 2;
        } else {
            arr.push(binary_string.charCodeAt(i));
            i += 1;
        }
    }
    return arr.map(e => to(e)).join('U');
};

const decompressFromHuH = (msg) => {
    const en_li = msg.split('U').map(e => {
        const num = from(e);
        if (num < 0) throw new Error("Invalid format");
        return num;
    });

    if (en_li[0] > 255) {
        let binary_str = "";
        for (let e of en_li) {
            if (e >= 65792) {
                e -= 65792;
                binary_str += String.fromCharCode((e >> 16) & 255, (e >> 8) & 255, e & 255);
            } else if (e >= 256) {
                e -= 256;
                binary_str += String.fromCharCode((e >> 8) & 255, e & 255);
            } else {
                binary_str += String.fromCharCode(e);
            }
        }
        return window.btoa(binary_str);
    } else {
        return en_li.map(num => String.fromCharCode(num)).join('');
    }
};

// ---------- Encrypt / Decrypt ----------
// Returns the AES key for the current mode, or null after showing why not.
const currentKey = (verb) => {
    if (mode === 'password') {
        const pass = passIn.value;
        if (!pass) {
            passIn.focus();
            showError(FACE.wave, 'Password, please',
                verb === 'encrypt' ? 'Add the one you both know, or switch to Session.'
                                   : 'Type the one it was locked with.');
            return null;
        }
        return pass;
    }
    if (!session) {
        showError(FACE.alarm, 'No session yet',
            verb === 'encrypt' ? 'Pair with someone first, then lock away.'
                               : 'Pair with someone first, then unlock away.');
        return null;
    }
    return ss;
};

const encrypt = async () => {
    if (attached) { await encryptAttached(); return; }
    const msg = textIn.value;
    if (!msg.trim()) {
        textIn.focus();
        showError(FACE.alarm, 'Enter something first…', 'I mean, we can encrypt nothing, and that’s nothing.');
        return;
    }
    const key = currentKey('encrypt');
    if (key === null) return;
    try {
        const en = CryptoJS.AES.encrypt(msg, key);
        showResult(compressToHuH(en.toString()), 'huh', mode);
    } catch (err) {
        showError(FACE.dead, 'Something went wrong', err.message || 'An unknown error occurred.');
    }
};

// Locks the attached file: header + bytes → AES → huH → a .huh to save.
// The bytes go to AES as they are; base64 first would only make the huH bigger.
const encryptAttached = async () => {
    const key = currentKey('encrypt');
    if (key === null) return;
    const { file } = attached;
    setWorking(encryptBtn, 'Locking…');
    await nextPaint();
    try {
        const bytes = new Uint8Array(await file.arrayBuffer());
        const plain = packFile(bytes, file.name, file.type);
        const en = CryptoJS.AES.encrypt(bytesToWordArray(plain), key);
        showHuhFileResult(compressToHuH(en.toString()), `${file.name}.huh`, mode);
    } catch (err) {
        showError(FACE.dead, 'Something went wrong', err.message || 'An unknown error occurred.');
    } finally {
        clearWorking(encryptBtn);
    }
};

const decrypt = async () => {
    let msg;
    if (attached) {
        if (attached.kind !== 'huh') {
            showError(FACE.huh, 'That isn’t a huH', 'Only a .huh can be unlocked. Hit Encrypt to lock this file instead.');
            return;
        }
        msg = attached.text;
    } else {
        msg = textIn.value.trim();
        if (!msg) {
            textIn.focus();
            showError(FACE.alarm, 'Enter something first…', 'Why waste processing power on an empty string?');
            return;
        }
    }
    const key = currentKey('decrypt');
    if (key === null) return;

    const huh = /^[huHU]+$/.test(msg);
    // Raw CryptoJS output (the pre-huH format) always starts with "Salted__" in base64.
    const legacy = !huh && /^U2FsdGVk/.test(msg);
    // File-sized huHs take a moment; say so on the button while they run.
    const heavy = msg.length > 200000;
    if (heavy) { setWorking(decryptBtn, 'Unlocking…'); await nextPaint(); }
    try {
        let decoded = msg;
        if (huh) {
            try {
                decoded = decompressFromHuH(msg);
            } catch (_) {
                showError(FACE.huh, 'That isn’t a huH', 'Paste the whole thing. It’s only h, u, H and U.');
                return;
            }
        }

        let words = null;
        try {
            words = CryptoJS.AES.decrypt(decoded, key);
        } catch (_) {
            words = null;
        }

        let out = '';
        if (words && words.sigBytes > 0) {
            // A file payload starts with NUL, which typed text never does.
            if (words.sigBytes > FILE_MAGIC.length && (words.words[0] >>> 24) === 0) {
                const file = unpackFile(wordArrayToBytes(words));
                if (file) { showUnlockedFileResult(file, mode); return; }
            }
            try {
                out = CryptoJS.enc.Utf8.stringify(words);
            } catch (_) {
                out = '';
            }
        }

        if (!out) {
            if (!huh && !legacy) {
                showError(FACE.huh, 'That isn’t a huH', 'Paste the whole thing. It’s only h, u, H and U.');
            } else if (mode === 'password') {
                showError(FACE.grump, 'Invalid password, mate!', 'You frogot it didn’t you.');
            } else {
                showError(FACE.grump, 'Session keys don’t match', 'This huH was locked somewhere else. Try editing the session.');
            }
            return;
        }
        showResult(out, 'plain', mode);
    } finally {
        if (heavy) clearWorking(decryptBtn);
    }
};

const clearText = () => {
    detachFile();
    textIn.value = '';
    textIn.focus();
    clearError();
    refreshCompose();
};

// ---------- Byte helpers ----------
const Uint8ToString = (u8a) => {
    const CHUNK_SZ = 0x8000;
    const c = [];
    for (let i = 0; i < u8a.length; i += CHUNK_SZ) {
        c.push(String.fromCharCode.apply(null, u8a.subarray(i, i + CHUNK_SZ)));
    }
    return c.join("");
};

const StringToUnit8 = (base64) => {
    const binaryString = atob(base64);
    const length = binaryString.length;
    const bytes = new Uint8Array(length);
    for (let i = 0; i < length; i++) {
        bytes[i] = binaryString.charCodeAt(i);
    }
    return bytes.buffer;
};

// CryptoJS 3.1.2 (the AES rollup the page loads) has no typed-array support,
// so file bytes cross into and out of its WordArray format by hand.
const bytesToWordArray = (u8) => {
    const len = u8.length;
    const words = new Array((len + 3) >>> 2);
    for (let i = 0, w = 0; i < len; i += 4, w++) {
        words[w] = ((u8[i] << 24) | (u8[i + 1] << 16) | (u8[i + 2] << 8) | u8[i + 3]) | 0;
    }
    return CryptoJS.lib.WordArray.create(words, len);
};

const wordArrayToBytes = (wa) => {
    const { words, sigBytes } = wa;
    const u8 = new Uint8Array(sigBytes);
    for (let i = 0; i < sigBytes; i++) {
        u8[i] = (words[i >>> 2] >>> (24 - (i % 4) * 8)) & 0xff;
    }
    return u8;
};

// ---------- File payload ----------
// A locked file is: MAGIC · 2-byte header length · JSON header {n: name, t: type} · bytes.
// MAGIC starts with NUL, which no typed message can, so unlock can tell files from text.
const FILE_MAGIC = [0x00, 0x68, 0x75, 0x48, 0x46]; // NUL h u H F

const packFile = (bytes, name, type) => {
    const header = new TextEncoder().encode(JSON.stringify({ n: name, t: type || '' }));
    if (header.length > 0xffff) throw new Error('That filename is too long.');
    const base = FILE_MAGIC.length + 2;
    const out = new Uint8Array(base + header.length + bytes.length);
    out.set(FILE_MAGIC, 0);
    out[FILE_MAGIC.length] = header.length >>> 8;
    out[FILE_MAGIC.length + 1] = header.length & 0xff;
    out.set(header, base);
    out.set(bytes, base + header.length);
    return out;
};

// Returns { name, type, bytes } for a file payload, or null for anything else.
const unpackFile = (u8) => {
    const base = FILE_MAGIC.length + 2;
    if (u8.length < base) return null;
    for (let i = 0; i < FILE_MAGIC.length; i++) {
        if (u8[i] !== FILE_MAGIC[i]) return null;
    }
    const hlen = (u8[FILE_MAGIC.length] << 8) | u8[FILE_MAGIC.length + 1];
    if (base + hlen > u8.length) return null;
    let header;
    try {
        header = JSON.parse(new TextDecoder().decode(u8.subarray(base, base + hlen)));
    } catch (_) {
        return null;
    }
    if (!header || typeof header.n !== 'string') return null;
    return {
        name: header.n,
        type: typeof header.t === 'string' ? header.t : '',
        bytes: u8.subarray(base + hlen),
    };
};

// ---------- Attachments ----------
const detachFile = () => {
    attached = null;
    if (attachUrl) { URL.revokeObjectURL(attachUrl); attachUrl = null; }
    attachThumb.removeAttribute('src');
    refreshCompose();
};

// Takes a picked / dropped / pasted file. A .huh (or any text file that is
// only h, u, H and U) is held as a huH to unlock; anything else is a file to lock.
const attachFile = async (file) => {
    if (!file) return;
    clearError();

    let kind = 'file';
    let text = null;
    const maybeHuh = /\.(huh|txt)$/i.test(file.name) || file.type === 'text/plain';
    if (maybeHuh && file.size <= MAX_HUH_BYTES) {
        const t = await file.text();
        if (isHuhText(t)) { kind = 'huh'; text = t.trim(); }
    }
    if (kind === 'file' && file.size > MAX_FILE_BYTES) {
        showError(FACE.grump, 'That file’s a bit much',
            `Files up to ${fmtSize(MAX_FILE_BYTES)} only. huH makes them five times bigger, so keep it small.`);
        return;
    }

    detachFile();
    attached = { file, kind, text };

    const isImage = /^image\//.test(file.type);
    if (isImage) {
        attachUrl = URL.createObjectURL(file);
        attachThumb.src = attachUrl;
    }
    attachThumb.hidden = !isImage;
    attachGlyph.hidden = isImage;
    attachGlyphText.hidden = kind !== 'huh';
    attachGlyphIcon.toggleAttribute('hidden', kind === 'huh');
    attachName.textContent = file.name;
    attachMeta.textContent = kind === 'huh'
        ? `${fmtSize(file.size)} of huH · ready to unlock`
        : `${fmtSize(file.size)}${file.type ? ' · ' + file.type : ''} · ready to lock`;

    refreshCompose();
    announce(`Attached ${file.name}`);
};

// Busy state for file-sized runs: the button says what it's doing and both
// actions wait. nextPaint() lets that label show before the work starts.
const setWorking = (btn, text) => {
    const span = btn.querySelector('span');
    btn._workingOriginal = span.textContent;
    span.textContent = text;
    encryptBtn.disabled = decryptBtn.disabled = true;
};

const clearWorking = (btn) => {
    const span = btn.querySelector('span');
    if (btn._workingOriginal != null) span.textContent = btn._workingOriginal;
    btn._workingOriginal = null;
    encryptBtn.disabled = decryptBtn.disabled = false;
};

const nextPaint = () => new Promise((resolve) => requestAnimationFrame(() => setTimeout(resolve, 0)));

// ---------- Session (pairing) ----------
const reset_pqc = () => {
    pkR = skR = myCt = myEncapSs = ss = null;
    session = false;
    sessionName = otherPkStored = otherCtStored = null;
    refreshSessionPanel();
};

const endSession = () => {
    reset_pqc();
    clearError();
    startPairBtn.focus();
    announce('Session ended');
};

// Opens the pairing dialog. In edit mode the existing keypair is reused (so the
// user's public key stays the same); only the other person's key / reply are
// pre-filled for correction.
const createSession = async (isEdit = false) => {
    if (!isEdit) {
        myCt = myEncapSs = null;
        try {
            [pkR, skR] = await mlkem_generateKeyPair();
        } catch (err) {
            showError(FACE.dead, 'Key generation failed', err.message || 'An error occurred.');
            return;
        }
    }
    await openPairDialog(isEdit);
};

const closeDialog = () => new Promise((resolve) => {
    if (!dialogEl.open) { resolve(); return; }
    dialogEl.classList.add('closing');
    setTimeout(() => {
        dialogEl.classList.remove('closing');
        dialogEl.close();
        document.body.classList.remove('modal-open');
        resolve();
    }, 180);
});

const setCtStatus = (text) => { ctStatus.textContent = text; };

const openPairDialog = (isEdit) => {
    return new Promise((resolve) => {
        dialogTitle.textContent = isEdit ? 'Edit the session' : 'Pair a session';
        startBtn.querySelector('span').textContent = isEdit ? 'Update' : 'Pair';
        clearDialogError();

        // "Copy my key" is ready immediately
        const myPkB64 = btoa(Uint8ToString(pkR));
        copyPkLink.onclick = () => copyText(myPkB64, copyPkLink, showDialogError);

        // Pre-fill + reset visibility
        setVisibility(otherPkIn, false);
        setVisibility(otherCtIn, false);
        otherPkIn.value = isEdit ? (otherPkStored || '') : '';
        otherCtIn.value = isEdit ? (otherCtStored || '') : '';

        // Restore the saved reply if editing with an existing CT
        const origMyCt      = myCt;
        const origMyEncapSs = myEncapSs;
        const enableCt = () => {
            const myCtB64 = btoa(Uint8ToString(myCt));
            copyCtLink.onclick = () => copyText(myCtB64, copyCtLink, showDialogError);
            copyCtLink.disabled = false;
            setCtStatus('Ready to send');
        };
        const disableCt = (status) => {
            copyCtLink.disabled = true;
            copyCtLink.onclick = null;
            setCtStatus(status);
        };
        if (isEdit && myCt) enableCt();
        else disableCt('Needs their key first');

        // Auto-generate the reply (CT) whenever the other person's key changes.
        // Any reply made for an earlier key is void the moment the key changes,
        // so Pair's "Not ready yet" guard holds until the new one is ready.
        let ctGenTimer = null;
        let closed = false;
        let generation = 0;
        const handlePkInput = () => {
            clearTimeout(ctGenTimer);
            clearDialogError();
            myCt = myEncapSs = null;
            const val = otherPkIn.value.trim();
            if (!val) { disableCt('Needs their key first'); return; }
            disableCt('Generating…');
            const mine = ++generation;
            ctGenTimer = setTimeout(async () => {
                try {
                    const otherPk = new Uint8Array(StringToUnit8(val));
                    const [ctBytes, ssBytes] = await mlkem_encap(otherPk);
                    if (closed || mine !== generation) return; // superseded meanwhile
                    myCt      = ctBytes;
                    myEncapSs = ssBytes;
                    enableCt();
                } catch (_) {
                    if (closed || mine !== generation) return;
                    myCt = myEncapSs = null;
                    disableCt('That key doesn’t look right');
                }
            }, 400);
        };
        otherPkIn.addEventListener('input', handlePkInput);
        otherCtIn.addEventListener('input', clearDialogError);
        // If editing with a stored key but no reply yet, generate now
        if (isEdit && otherPkStored && !myCt) handlePkInput();

        const cleanup = () => {
            closed = true;
            startBtn.removeEventListener('click', onStart);
            cancelBtn.removeEventListener('click', onCancel);
            dialogEl.removeEventListener('cancel', onEsc);
            otherPkIn.removeEventListener('input', handlePkInput);
            otherCtIn.removeEventListener('input', clearDialogError);
            clearTimeout(ctGenTimer);
        };

        const onStart = async () => {
            if (!otherPkIn.value.trim() || !otherCtIn.value.trim()) {
                showDialogError(FACE.wave, 'Missing a code', 'Paste both the other person’s key and their reply.');
                (otherPkIn.value.trim() ? otherCtIn : otherPkIn).focus();
                return;
            }
            if (!myCt || !myEncapSs) {
                showDialogError(FACE.wave, 'Not ready yet', 'Your reply is still generating. Give it a moment after pasting their key.');
                return;
            }
            cleanup();
            await closeDialog();
            const prevPk = otherPkStored;
            const prevCt = otherCtStored;
            try {
                otherPkStored = otherPkIn.value.trim();
                otherCtStored = otherCtIn.value.trim();

                const otherCtBytes = new Uint8Array(StringToUnit8(otherCtStored));
                const myDecapSs    = await mlkem_decap(otherCtBytes, skR);

                // XOR is commutative: both parties arrive at the same combined key
                const combined = new Uint8Array(myEncapSs.length);
                for (let i = 0; i < combined.length; i++) {
                    combined[i] = myEncapSs[i] ^ myDecapSs[i];
                }
                ss          = btoa(Uint8ToString(combined));
                sessionName = deriveSessionName(combined);
                session     = true;

                refreshSessionPanel();
                clearError();
                sessionNameEl.focus({ preventScroll: true });
                announce(`Paired as ${sessionName}`);
                resolve(true);
            } catch (err) {
                // Leave whatever was paired before exactly as it was.
                otherPkStored = prevPk;
                otherCtStored = prevCt;
                myCt      = origMyCt;
                myEncapSs = origMyEncapSs;
                showError(FACE.dead, 'Pairing failed', err.message || 'Check the codes and try again.');
                resolve(false);
            }
        };

        const onCancel = async () => {
            // Restore the reply if the user changed the key field then cancelled;
            // a fresh attempt leaves nothing behind.
            myCt      = isEdit ? origMyCt : null;
            myEncapSs = isEdit ? origMyEncapSs : null;
            cleanup();
            await closeDialog();
            resolve(false);
        };

        const onEsc = (e) => {
            e.preventDefault(); // we close with our own animation
            onCancel();
        };

        startBtn.addEventListener('click', onStart);
        cancelBtn.addEventListener('click', onCancel);
        dialogEl.addEventListener('cancel', onEsc);

        dialogEl.showModal();
        document.body.classList.add('modal-open');
        copyPkLink.focus();
    });
};

const editSession = () => createSession(true);
const startNewSession = () => createSession(false);

// ---------- Password visibility ----------
const setVisibility = (input, shown) => {
    const btn = document.querySelector(`.pw-toggle[data-for="${input.id}"]`);
    input.type = shown ? 'text' : 'password';
    if (!btn) return;
    // The icons are <svg>, which has no `hidden` property: toggle the attribute.
    btn.querySelector('.icon-hidden').toggleAttribute('hidden', shown);
    btn.querySelector('.icon-shown').toggleAttribute('hidden', !shown);
    const what = btn.dataset.what || 'password';
    btn.setAttribute('aria-label', (shown ? 'Hide ' : 'Show ') + what);
};

document.querySelectorAll('.pw-toggle').forEach((btn) => {
    const input = $(btn.dataset.for);
    if (!input) return;
    // Remember the noun from the initial label ("Show password" → "password")
    btn.dataset.what = (btn.getAttribute('aria-label') || 'Show password').replace(/^Show\s+/i, '');
    btn.addEventListener('click', () => setVisibility(input, input.type === 'password'));
});

// ---------- Wiring ----------
textIn.addEventListener('input', () => { clearError(); refreshCompose(); });
passIn.addEventListener('input', clearError);
clearBtn.addEventListener('click', clearText);

modePwBtn.addEventListener('click', () => setMode('password'));
modeSessBtn.addEventListener('click', () => setMode('session'));

encryptBtn.addEventListener('click', encrypt);
decryptBtn.addEventListener('click', decrypt);

copyBtn.addEventListener('click', () => { if (resultText != null) copyText(resultText, copyBtn); });
saveBtn.addEventListener('click', () => {
    if (!resultBlob) return;
    downloadBlob(resultBlob, resultName);
    flashLabel(saveBtn, 'Saved!');
    announce(`Saved ${resultName}`);
});

// Files in: the picker, a drop onto the compose card, or a paste into the message box.
attachBtn.addEventListener('click', () => fileInput.click());
fileInput.addEventListener('change', () => {
    attachFile(fileInput.files[0]);
    fileInput.value = '';
});
attachRemove.addEventListener('click', () => {
    detachFile();
    clearError();
    textIn.focus();
});
textIn.addEventListener('paste', (e) => {
    const f = e.clipboardData && e.clipboardData.files && e.clipboardData.files[0];
    if (f) { e.preventDefault(); attachFile(f); }
});

const dragHasFiles = (e) => !!e.dataTransfer && Array.from(e.dataTransfer.types || []).includes('Files');
let dragDepth = 0; // enter/leave fire for every child, so count them
composeEl.addEventListener('dragenter', (e) => {
    if (!dragHasFiles(e)) return;
    e.preventDefault();
    dragDepth++;
    composeEl.classList.add('dragging');
});
composeEl.addEventListener('dragover', (e) => {
    if (!dragHasFiles(e)) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = 'copy';
});
composeEl.addEventListener('dragleave', (e) => {
    if (!dragHasFiles(e)) return;
    if (--dragDepth <= 0) { dragDepth = 0; composeEl.classList.remove('dragging'); }
});
composeEl.addEventListener('drop', (e) => {
    if (!dragHasFiles(e)) return;
    e.preventDefault();
    dragDepth = 0;
    composeEl.classList.remove('dragging');
    attachFile(e.dataTransfer.files[0]);
});
// A file dropped anywhere else on the page must not navigate away from the app.
document.addEventListener('dragover', (e) => { if (dragHasFiles(e)) e.preventDefault(); });
document.addEventListener('drop', (e) => { if (dragHasFiles(e)) e.preventDefault(); });

copyKeyBtn.addEventListener('click', () => {
    if (!pkR) return;
    copyText(btoa(Uint8ToString(pkR)), copyKeyBtn);
});
copyReplyBtn.addEventListener('click', () => {
    if (!myCt) return;
    copyText(btoa(Uint8ToString(myCt)), copyReplyBtn);
});
editSessBtn.addEventListener('click', editSession);
endSessBtn.addEventListener('click', endSession);
startPairBtn.addEventListener('click', startNewSession);

addEventListener('resize', () => { if (!resultOut.hidden && !result.hidden) resizeResult(); });
result.addEventListener('scroll', updateResultFade, { passive: true });

// Click bounce on every button
document.querySelectorAll('button').forEach(btn => {
    btn.addEventListener('click', function () {
        if (this.disabled) return;
        this.classList.remove('bounce-anim');
        void this.offsetWidth; // restart the animation
        this.classList.add('bounce-anim');
    });
});

// ---------- Init ----------
setMode('password');
refreshCompose();
refreshSessionPanel();
textIn.focus({ preventScroll: true });
