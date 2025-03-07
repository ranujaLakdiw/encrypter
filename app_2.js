const code = [
    'h',
    'u',
    'H',
]

var [pkR, skR, ct, ssS, ssR, secret, session] = [0, 0, 0, 0, 0, 0, false]


document.getElementById('text').focus();

const c_btn = document.querySelector('#copy-btn');
const result = document.querySelector("#result");

const activateBtn = () => {
    c_btn.classList.remove('disable');
};

const deactivateBtn = () => {
    c_btn.classList.add('disable');
};

const animateResult = () => {
    result.style.animation = "2s bounceIn";
    setTimeout(() => {
        result.style.animation = "";
    }, 2000);
};

const animateErr = () => {
    let temp = document.querySelector('.alert-box');
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

const resetPass = () => {
    document.querySelectorAll('input').forEach((el) => {
        el.type = "password";
    });
    document.querySelectorAll('.password-visibility').forEach((el) => {
        el.innerHTML = eyeClosed;
    });
}

const alertMsg = (text_1, text_2) => {
    let alert = document.querySelector('#alert');
    let alert_text_1 = document.querySelector('#alert .body_1');
    let alert_text_2 = document.querySelector('#alert .body_2');
    let alert_btn = document.querySelector('#alert button');

    let btn_click = () => {
        alert.classList.remove('appear');
        alert_text_1.innerHTML = '';
        alert_text_2.innerHTML = '';
        alert_btn.removeEventListener('click', btn_click);
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
    [pkR, skR, ct, ssS, ssR, secret, session] = [0, 0, 0, 0, 0, 0, false]
    document.getElementById('pqc').innerHTML = '';
    document.getElementById('pub-key-button').classList.remove('disable');
}

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
        let en_li = (en.toString()).split('').map(el => el.charCodeAt(0));
        if (en_li.includes(-1)) {
            alertMsg("Invalid formats !", "The format used is unsupported by us, sorry for any inconvenience");
            out = "";
            deactivateBtn();
        } else {

            out = en_li.map(e => {
                return to(e);
            }).join('U');
            activateBtn();
            animateResult();
        }
        result.innerHTML = out;
        textAreaResize();
        c_btn.innerHTML = "copy";

    } else {
        alertMsg("Enter something first...", "I mean we can encrypt nothing, and that's nothing")
    }

}

const decrypt = () => {
    let out = "";
    let esc = false;
    let msg = document.querySelector('#text').value;
    let pass = document.querySelector('#pass').value;
    if (pass == '') {
        if (session) decryptAD();
    }
    unCancelSession();
    let msg_li = msg.split('U');
    if (msg_li.length >= 1 && msg_li[0] != "") {
        try {
            out = msg_li.map(e => {
                let num = from(e);
                if (num < 0) {
                    esc = true;
                }
                return String.fromCharCode(num);
            }).join('');

            if (esc) {
                alertMsg("Invalid ecrypted format !", "haven't your mother taught you better");
                out = "";
                deactivateBtn();
            } else {
                activateBtn();
                animateResult();
            }
            out = CryptoJS.AES.decrypt(out, pass).toString(CryptoJS.enc.Utf8);
            if (out == "") {
                alertMsg("Invalid password mate !", "You frogot it didn't you （︶^︶）");
            }
            result.innerHTML = out;
            textAreaResize();
            c_btn.innerHTML = "copy";
        } catch (err) {
            alertMsg("Error", "An unknown error has occured");
        }
    } else {
        alertMsg("Enter something first...", "Why waste processing power on an empty string?")
    }
}

var copy_result = async () => {
    if (window.isSecureContext && navigator.clipboard) {
        await navigator.clipboard.writeText(result.value);
        c_btn.innerHTML = "copied";
    } else {
        alert('The copy button might not work !","Sorry for any inconviences...');
        try {
            result.select();
            result.setSelectionRange(0, 99999);
            document.execCommand('copy');
            c_btn.innerHTML = "copied";
        } catch (err) {
            console.error('Unable to copy to clipboard', err);
        }
    }
};

var copy = async (txt) => {
    if (window.isSecureContext && navigator.clipboard) {
        await navigator.clipboard.writeText(txt);
    } else {
        alert('The copy button might not work !","Sorry for any inconviences...');
        try {
            let copier = document.getElementById('copy');
            copier.innerHTML = txt;
            copier.select();
            copier.setSelectionRange(0, 99999);
            document.execCommand('copy');
            copier.innerHTML = ''
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

const encryptAD = async () => {

    if (!session) {
        try {
            [pkR, skR, ct, ssS] = await doMlKem(pkR, skR, ct);
            document.getElementById('pqc').innerHTML = `private key in session;<br/><br/><a href="javascript:void(0);" onclick="copy('${btoa(Uint8ToString(skR))}')">copy public key</a><br/><br/><a href="javascript:void(0);" onclick="copy('${btoa(Uint8ToString(ct))}')">copy CT</a>`;
            secret = ssS.toString();
            session = true
            cancelSession();
        } catch (err) {
            console.log("session online");
        }
    } else console.log("session online");



    if (document.getElementById("text").value != '') {
        var en = CryptoJS.AES.encrypt(
            document.getElementById("text").value,
            secret
        );
        let en_li = (en.toString()).split('').map(el => el.charCodeAt(0));
        if (en_li.includes(-1)) {
            alertMsg("Invalid formats !", "The format used is unsupported by us, sorry for any inconvenience");
            out = "";
            deactivateBtn();
        } else {

            out = en_li.map(e => {
                return to(e);
            }).join('U');
            activateBtn();
            animateResult();
        }
        result.innerHTML = out;
        textAreaResize();
        c_btn.innerHTML = "copy";

    } else {
        alertMsg("Enter something first...", "I mean we can encrypt nothing, and that's nothing")
    }
}

const decrytpAD_2 = async () => {
    let msg = document.querySelector('#text').value;
    let msg_li = msg.split('U');
    let esc = false;
    if (msg_li.length >= 1 && msg_li[0] != "") {
        try {
            let de = msg_li.map(e => {
                let num = from(e);
                if (num < 0) {
                    esc = true;
                }
                return String.fromCharCode(num);
            }).join('');

            if (esc) {
                alertMsg("Invalid ecrypted format !", "haven't your mother taught you better");
                out = "";
                deactivateBtn();
            } else {
                activateBtn();
                animateResult();
            }
            let out = CryptoJS.AES.decrypt(de, secret).toString(CryptoJS.enc.Utf8);
            if (out == "") {
                alertMsg("Invalid public key mate !", "You copied something else didn't you");
            }
            result.innerHTML = out;
            textAreaResize();
            c_btn.innerHTML = "copy";
        } catch (err) {
            alertMsg("Error", err);
        }
    } else {
        alertMsg("Enter something first...", "Why waste processing power on an empty string?")
    }


}

const decryptAD = async () => {
    let alert = document.getElementById("alert-pub-key");
    let alert_start_btn = document.querySelector('#start-session');
    let alert_cancel_btn = document.querySelector('#cancel-session');
    let pub_key = document.getElementById("pub-key");
    let ct_key = document.getElementById("ct");

    if (!session) {
        resetPass();
        let btn_click = async () => {
            alert.classList.remove('appear');
            alert_start_btn.removeEventListener('click', btn_click);
            if (pub_key.value == '' || ct_key.value == '') {
                alertMsg("Error...", "you are missing entries!");
                return
            }
            document.getElementById('pub-key-button').classList.add('disable');
            skR = new Uint8Array(StringToUnit8(pub_key.value));
            ct = new Uint8Array(StringToUnit8(ct_key.value));

            [ssR] = await doMlKem(pkR, skR, ct);
            secret = ssR.toString();
            document.getElementById('pqc').innerHTML = `public key in session;</br></br><a href="javascript:void(0);" onclick="copy('${btoa(Uint8ToString(skR))}')">copy public key</a><br/><br/><a href="javascript:void(0);" onclick="copy('${btoa(Uint8ToString(ct))}')">copy CT</a>`;
            session = true;
            decrytpAD_2();
        }
        let btn_cancel_click = async () => {
            alert.classList.remove('appear');
            alert_cancel_btn.removeEventListener('click', btn_cancel_click);
        }
        alert.classList.add('appear');
        alert_start_btn.addEventListener('click', btn_click);
        alert_cancel_btn.addEventListener('click', btn_cancel_click);
    } else {
        console.log("session online");
        decrytpAD_2();
    }
}

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
    isShowPass = !isShowPass
}

const pubKey = document.getElementById("pub-key-button");
pubKey.addEventListener('click', decryptAD);

function unCancelSession() {
    reset_pqc();
    pubKey.addEventListener('click', decryptAD);
    pubKey.removeEventListener('click', unCancelSession);
    pubKey.innerHTML = "Enter Public Key";
}

function cancelSession() {
    pubKey.innerHTML = "Cancel Session";
    pubKey.removeEventListener('click', decryptAD);
    pubKey.addEventListener('click', unCancelSession);

}