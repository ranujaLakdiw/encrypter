const code = [
    'h',
    'u',
    'H',
]

// --- PQC state ---
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

document.getElementById('text').focus();

const c_btn = document.querySelector('#copy-btn');
const result = document.querySelector("#result");

const activateBtn = () => {
    c_btn.classList.remove('disable');
};

const deactivateBtn = () => {
    c_btn.classList.add('disable');
};

const hideAlert = (el, callback) => {
    el.classList.add('disappear');
    setTimeout(() => {
        el.classList.remove('appear', 'disappear');
        if (callback) callback();
    }, 200);
};

const animateResult = () => {
    result.style.animation = "none";
    void result.offsetWidth; // force reflow so the animation restarts each time
    result.style.animation = "2s bounceIn";
    setTimeout(() => {
        result.style.animation = "";
    }, 2000);
};

const animateErr = () => {
    let temp = document.querySelector('.alert-box');
    temp.style.animation = "none";
    void temp.offsetWidth; // force reflow
    temp.style.animation = ".5s shake";
    clearText();
    setTimeout(() => {
        temp.style.animation = "";
    }, 500);
};

const textAreaResize = () => {
    result.style.height = '1px';
    result.style.height = (result.scrollHeight) + 'px';
};

const alertMsg = (text_1, text_2) => {
    let alert = document.querySelector('#alert');
    let alert_text_1 = document.querySelector('#alert .body_1');
    let alert_text_2 = document.querySelector('#alert .body_2');
    let alert_btn = document.querySelector('#alert button');

    let btn_click = () => {
        alert_btn.removeEventListener('click', btn_click);
        hideAlert(alert, () => {
            alert_text_1.innerHTML = '';
            alert_text_2.innerHTML = '';
        });
    };

    alert_text_1.innerHTML = text_1;
    alert_text_2.innerHTML = text_2;
    alert.classList.add('appear');
    alert_btn.addEventListener('click', btn_click);
    alert_btn.focus();
    animateErr();
};

const getIndex = (str, list) => {
    return list.findIndex((e) => {
        return str == e;
    });
};

const getPower = (num) => {
    let count = 0;
    while (num >= 0) {
        if (num < 3 ** count)
            return count - 1;
        count++;
    }
}

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
    return (out);
};

const from = (i) => {
    let num = 0;
    let li = i.split('');
    let p = li.length - 1;
    for (e of li) {
        index = getIndex(e, code);
        if (index < 0) {
            return -1;
        }
        num += index * (3 ** p);
        p--;
    };
    return (num);
};

const reset_pqc = () => {
    pkR = skR = myCt = myEncapSs = ss = null;
    session = false;
    sessionName = otherPkStored = otherCtStored = null;
    document.getElementById('pqc').innerHTML = '';
    document.getElementById('pub-key-button').classList.remove('disable');
}

const compressToHuH = (base64_str) => {
    let binary_string = window.atob(base64_str);
    let arr = [];
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
    let en_li = msg.split('U').map(e => {
        let num = from(e);
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

const encrypt = async () => {
    let out = "";
    let msg = document.querySelector('#text').value;
    let pass = document.querySelector('#pass').value;

    if (pass == '') {
        encryptAD();
        return;
    }
    unCancelSession();
    if (msg != '') {
        var en = CryptoJS.AES.encrypt(msg, pass);
        out = compressToHuH(en.toString());
        activateBtn();
        animateResult();
        result.value = out;
        textAreaResize();
        c_btn.innerHTML = "copy";

    } else {
        alertMsg("Enter something first...", "I mean we can encrypt nothing, and that's nothing")
    }

}

const decrypt = () => {
    let out = "";
    let msg = document.querySelector('#text').value;
    let pass = document.querySelector('#pass').value;
    if (pass == '') {
        decryptAD();
        return;
    }
    unCancelSession();
    if (msg != '') {
        try {
            let decoded = msg;
            if (/^[huHU]+$/.test(msg)) {
                decoded = decompressFromHuH(msg);
            }
            out = CryptoJS.AES.decrypt(decoded, pass).toString(CryptoJS.enc.Utf8);
            if (!out) {
                alertMsg("Invalid password mate !", "You frogot it didn't you （︶^︶）");
                deactivateBtn();
            } else {
                activateBtn();
                animateResult();
                result.value = out;
                textAreaResize();
                c_btn.innerHTML = "copy";
            }
        } catch (err) {
            alertMsg("Error", "An unknown error has occured");
            deactivateBtn();
        }
    } else {
        alertMsg("Enter something first...", "Why waste processing power on an empty string?")
    }
}

var copy_result = async () => {
    let setCopied = () => {
        c_btn.innerHTML = "copied";
        setTimeout(() => { if (c_btn.innerHTML === "copied") c_btn.innerHTML = "copy"; }, 2000);
    };

    if (window.isSecureContext && navigator.clipboard) {
        await navigator.clipboard.writeText(result.value);
        setCopied();
    } else {
        alertMsg('Hold up!', "The copy button might not work in this browser setting... Sorry for any inconveniences.");
        try {
            result.select();
            result.setSelectionRange(0, 99999);
            document.execCommand('copy');
            setCopied();
        } catch (err) {
            console.error('Unable to copy to clipboard', err);
        }
    }
};

var copy = async (txt, element) => {
    let setCopied = () => {
        if (element) {
            let originalText = element.innerHTML;
            element.innerHTML = "copied!";
            setTimeout(() => { element.innerHTML = originalText; }, 2000);
        }
    };
    if (window.isSecureContext && navigator.clipboard) {
        await navigator.clipboard.writeText(txt);
        setCopied();
    } else {
        alertMsg('Hold up!', "The copy button might not work in this browser setting... Sorry for any inconveniences.");
        try {
            let copier = document.getElementById('copy');
            copier.innerHTML = txt;
            copier.select();
            copier.setSelectionRange(0, 99999);
            document.execCommand('copy');
            copier.innerHTML = '';
            setCopied();
        } catch (err) {
            console.error('Unable to copy to clipboard', err);
        }
    }
}

const clearText = () => {
    document.getElementById('text').focus();
    document.getElementById('text').value = '';
};

addEventListener('resize', () => textAreaResize());

const Uint8ToString = (u8a) => {
    var CHUNK_SZ = 0x8000;
    var c = [];
    for (var i = 0; i < u8a.length; i += CHUNK_SZ) {
        c.push(String.fromCharCode.apply(null, u8a.subarray(i, i + CHUNK_SZ)));
    }
    return c.join("");
}

const StringToUnit8 = (base64) => {
    const binaryString = atob(base64);
    const length = binaryString.length;
    const bytes = new Uint8Array(length);
    for (let i = 0; i < length; i++) {
        bytes[i] = binaryString.charCodeAt(i);
    }
    return bytes.buffer;
}

// --- PQC: Session dialog ---
// Opens the "Create / Edit Session" dialog. In edit mode the existing keypair
// is reused (so the user's public key stays the same), only the other person's
// pk / CT fields are pre-filled for correction.
const createSession = async (isEdit = false) => {
    if (!isEdit) {
        try {
            [pkR, skR] = await mlkem_generateKeyPair();
        } catch (err) {
            alertMsg("Key generation failed", err.message || "An error occurred");
            return;
        }
    }
    await openSessionDialog(isEdit);
};

const openSessionDialog = (isEdit) => {
    return new Promise((resolve) => {
        const dialogEl   = document.getElementById("alert-session");
        const titleEl    = document.getElementById("session-dialog-title");
        const startBtn   = document.getElementById("session-start-btn");
        const cancelBtn  = document.getElementById("session-cancel-btn");
        const otherPkIn  = document.getElementById("session-other-pk");
        const otherCtIn  = document.getElementById("session-other-ct");
        const copyPkLink = document.getElementById("copy-pk-link");
        const copyCtLink = document.getElementById("copy-ct-link");

        titleEl.textContent = isEdit ? "Edit Session" : "Create Session";
        startBtn.textContent = isEdit ? "Update Session" : "Create Session";

        // Wire up the "copy my public key" link immediately
        const myPkB64 = btoa(Uint8ToString(pkR));
        copyPkLink.onclick = () => copy(myPkB64);

        // Pre-fill + reset visibility
        otherPkIn.type = "password";
        otherCtIn.type = "password";
        otherPkIn.value = isEdit ? (otherPkStored || '') : '';
        otherCtIn.value = isEdit ? (otherCtStored || '') : '';

        // Restore saved CT link if editing with an existing CT
        const origMyCt      = myCt;
        const origMyEncapSs = myEncapSs;
        if (isEdit && myCt) {
            const myCtB64 = btoa(Uint8ToString(myCt));
            copyCtLink.onclick = () => copy(myCtB64);
            copyCtLink.classList.remove('disable');
        } else {
            copyCtLink.classList.add('disable');
            copyCtLink.onclick = null;
        }

        // Auto-generate CT whenever the other person's pk input changes
        let ctGenTimer = null;
        const handlePkInput = () => {
            clearTimeout(ctGenTimer);
            copyCtLink.classList.add('disable');
            copyCtLink.onclick = null;
            const val = otherPkIn.value.trim();
            if (!val) return;
            ctGenTimer = setTimeout(async () => {
                try {
                    const otherPk = new Uint8Array(StringToUnit8(val));
                    const [ctBytes, ssBytes] = await mlkem_encap(otherPk);
                    myCt      = ctBytes;
                    myEncapSs = ssBytes;
                    const myCtB64 = btoa(Uint8ToString(ctBytes));
                    copyCtLink.onclick = () => copy(myCtB64);
                    copyCtLink.classList.remove('disable');
                } catch (_) { /* invalid pk — leave CT disabled */ }
            }, 400);
        };
        otherPkIn.addEventListener('input', handlePkInput);
        // If editing with a stored pk but no CT yet, trigger generation
        if (isEdit && otherPkStored && !myCt) handlePkInput();

        const cleanup = () => {
            startBtn.removeEventListener('click', onStart);
            cancelBtn.removeEventListener('click', onCancel);
            otherPkIn.removeEventListener('input', handlePkInput);
            clearTimeout(ctGenTimer);
        };

        const onStart = async () => {
            if (!otherPkIn.value.trim() || !otherCtIn.value.trim()) {
                hideAlert(dialogEl, () => alertMsg("Missing fields", "Fill in both the other person's public key and their CT."));
                cleanup();
                resolve(false);
                return;
            }
            if (!myCt || !myEncapSs) {
                hideAlert(dialogEl, () => alertMsg("Not ready", "Your CT is still generating — wait a moment after pasting the other's public key."));
                cleanup();
                resolve(false);
                return;
            }
            cleanup();
            hideAlert(dialogEl, async () => {
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

                    updateSessionDisplay();
                    if (!isEdit) cancelSession();
                    resolve(true);
                } catch (err) {
                    alertMsg("Session error", err.message || "An error occurred. Check the keys and try again.");
                    resolve(false);
                }
            });
        };

        const onCancel = () => {
            // Restore CT state if the user changed the pk field then cancelled
            if (isEdit) {
                myCt      = origMyCt;
                myEncapSs = origMyEncapSs;
            }
            cleanup();
            hideAlert(dialogEl, () => resolve(false));
        };

        dialogEl.classList.add('appear');
        startBtn.addEventListener('click', onStart);
        cancelBtn.addEventListener('click', onCancel);
    });
};

const updateSessionDisplay = () => {
    const myPkB64  = btoa(Uint8ToString(pkR));
    const myCtB64  = btoa(Uint8ToString(myCt));
    document.getElementById('pqc').innerHTML =
        `Session "<b>${sessionName}</b>"` +
        `&nbsp;&nbsp;<a href="javascript:void(0);" onclick="copy('${myPkB64}')">copy public key</a>` +
        `&nbsp;&nbsp;<a href="javascript:void(0);" onclick="copy('${myCtB64}')">copy CT</a>` +
        `&nbsp;&nbsp;<a href="javascript:void(0);" onclick="editSession()">edit</a>`;
};

// editSession is called from an inline onclick in updateSessionDisplay
const editSession = () => createSession(true);

// --- PQC Encrypt / Decrypt ---
// Once a session is active, both parties share the same ss and can freely
// encrypt and decrypt with each other — no sender/receiver distinction.
const encryptAD = async () => {
    if (!session) {
        alertMsg("No session", "Create a session first.");
        return;
    }
    const msg = document.getElementById("text").value;
    if (msg === '') {
        alertMsg("Enter something first...", "I mean we can encrypt nothing, and that's nothing");
        return;
    }
    const en  = CryptoJS.AES.encrypt(msg, ss);
    const out = compressToHuH(en.toString());
    activateBtn();
    animateResult();
    result.value = out;
    textAreaResize();
    c_btn.innerHTML = "copy";
};

const decryptAD = async () => {
    if (!session) {
        alertMsg("No session", "Create a session first.");
        return;
    }
    const msg = document.querySelector('#text').value;
    if (msg === '') {
        alertMsg("Enter something first...", "Why waste processing power on an empty string?");
        return;
    }
    try {
        let decoded = msg;
        if (/^[huHU]+$/.test(msg)) {
            decoded = decompressFromHuH(msg);
        }
        const out = CryptoJS.AES.decrypt(decoded, ss).toString(CryptoJS.enc.Utf8);
        if (!out) {
            alertMsg("Decryption failed", "Session keys don't match. Try editing the session.");
            deactivateBtn();
        } else {
            activateBtn();
            animateResult();
            result.value = out;
            textAreaResize();
            c_btn.innerHTML = "copy";
        }
    } catch (err) {
        alertMsg("Error", err.message || "An error occurred during decryption.");
        deactivateBtn();
    }
};

const change_color = () => {
    let el = document.querySelector('.pass');
    let p = document.getElementById('pass');

    if (p.value != "") {
        el.style.boxShadow = "green 0 0 10px";
        el.style.border = "green 1px solid";
    }
    else {
        el.style.boxShadow = "red 0 0 10px";
        el.style.border = "red 1px solid";
    }
}

const eyeOpen = `
<img src="./img/eye-open.svg">
<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
    <circle cx="50" cy="50" r="10" fill="transparent" stroke="white">
        <animate attributeName="r" begin="0" dur="1s" from="0" to="100%"/>
    </circle>
</svg>`;
const eyeClosed = `
<img src="./img/eye-closed.svg">
<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
    <circle cx="50" cy="50" r="10" fill="transparent" stroke="white">
        <animate attributeName="r" begin="0" dur="1s" from="0" to="100%"/>
    </circle>
</svg>`;

const showPass = (pass, el) => {
    if (pass.type == "text") {
        pass.type = "password";
        el.innerHTML = eyeClosed;
        el.querySelector('svg').classList.add("password-visibility-change");
    } else {
        pass.type = "text";
        el.innerHTML = eyeOpen;
        el.querySelector('svg').classList.add("password-visibility-change");
    }
}

const startNewSession = () => createSession(false);

const pqcBtn = document.getElementById("pub-key-button");
pqcBtn.addEventListener('click', startNewSession);

function unCancelSession() {
    reset_pqc();
    pqcBtn.addEventListener('click', startNewSession);
    pqcBtn.removeEventListener('click', unCancelSession);
    pqcBtn.innerHTML = "Create Session";
}

function cancelSession() {
    pqcBtn.innerHTML = "Cancel Session";
    pqcBtn.removeEventListener('click', startNewSession);
    pqcBtn.addEventListener('click', unCancelSession);
}

// Add bounce effect to all buttons
document.querySelectorAll('button').forEach(btn => {
    btn.addEventListener('click', function(e) {
        if (!this.classList.contains('disable')) {
            this.classList.remove('bounce-anim');
            void this.offsetWidth; // Trigger reflow
            this.classList.add('bounce-anim');
        }
    });
});
