// ─────────────────────────────────────────────────────────────
// Huh encrypter · app logic
// Crypto: AES (crypto-js) with either a shared password or an
// ML-KEM-1024 session secret. Output is "huH": base-3 text over
// the alphabet h / u / H, chunks joined with U.
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
var resultKind = null;   // 'huh' | 'plain' | null

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
    const huh = isHuhText(textIn.value);
    huhChip.hidden = !huh;
    textIn.classList.toggle('huh', huh);
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
const resizeResult = () => {
    result.style.height = '1px';
    result.style.height = result.scrollHeight + 'px';
};

const showResult = (value, kind, via) => {
    resultKind = kind;
    result.value = value;
    result.classList.toggle('huh', kind === 'huh');
    resultOut.hidden = false;
    resultEmpty.hidden = true;

    const locked = kind === 'huh';
    resultLabel.textContent = locked ? 'Locked · ready to send' : 'Unlocked';
    resultMeta.textContent = locked ? `${value.length} chars of huh` : `${value.length} chars`;
    const viaText = via === 'session' ? `in ${sessionName}` : 'with a password';
    cancelFlash(copyBtn);
    copyLabel.textContent = locked ? 'Copy huH' : 'Copy text';
    lockedWith.textContent = (locked ? 'locked ' : 'unlocked ') + viaText;
    resultMetaPhone.textContent = viaText;

    clearError();
    resizeResult();
    resultOut.classList.remove('pop');
    void resultOut.offsetWidth;
    resultOut.classList.add('pop');
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

const to = (i) => {
    let p = getPower(i);
    let out = '';
    while (p >= 0) {
        let x = parseInt(i / 3 ** p);
        if (0 <= x) {
            out = out + code[x];
            i = i - (x * 3 ** p);
        }
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

const encrypt = () => {
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

const decrypt = () => {
    const msg = textIn.value.trim();
    if (!msg) {
        textIn.focus();
        showError(FACE.alarm, 'Enter something first…', 'Why waste processing power on an empty string?');
        return;
    }
    const key = currentKey('decrypt');
    if (key === null) return;

    const huh = /^[huHU]+$/.test(msg);
    // Raw CryptoJS output (the pre-huH format) always starts with "Salted__" in base64.
    const legacy = !huh && /^U2FsdGVk/.test(msg);
    let decoded = msg;
    if (huh) {
        try {
            decoded = decompressFromHuH(msg);
        } catch (_) {
            showError(FACE.huh, 'That isn’t a huH', 'Paste the whole thing. It’s only h, u, H and U.');
            return;
        }
    }

    let out = '';
    try {
        out = CryptoJS.AES.decrypt(decoded, key).toString(CryptoJS.enc.Utf8);
    } catch (_) {
        out = '';
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
};

const clearText = () => {
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

copyBtn.addEventListener('click', () => copyText(result.value, copyBtn));

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

addEventListener('resize', () => { if (!resultOut.hidden) resizeResult(); });

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
