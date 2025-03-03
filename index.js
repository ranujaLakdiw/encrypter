const code = [
    'h',
    'u',
    'H',
]
// const key = 5;

const key = [250, 143];

// const letters = [
//     "A",
//     "B",
//     "C",
//     "D",
//     "E",
//     "F",
//     "G",
//     "H",
//     "I",
//     "J",
//     "K",
//     "L",
//     "M",
//     "N",
//     "O",
//     "P",
//     "Q",
//     "R",
//     "S",
//     "T",
//     "U",
//     "V",
//     "W",
//     "X",
//     "Y",
//     "Z",
//     "a",
//     "b",
//     "c",
//     "d",
//     "e",
//     "f",
//     "g",
//     "h",
//     "i",
//     "j",
//     "k",
//     "l",
//     "m",
//     "n",
//     "o",
//     "p",
//     "q",
//     "r",
//     "s",
//     "t",
//     "u",
//     "v",
//     "w",
//     "x",
//     "y",
//     "z",
//     "~",
//     "!",
//     "@",
//     "#",
//     "$",
//     "%",
//     "^",
//     "&",
//     "*",
//     "(",
//     ")",
//     "-",
//     "_",
//     "+",
//     "=",
//     "{",
//     "[",
//     "]",
//     "}",
//     "|",
//     ";",
//     ":",
//     "'",
//     '"',
//     "<",
//     ">",
//     ",",
//     ".",
//     "?",
//     "/",
//     "1",
//     "2",
//     "3",
//     "4",
//     "5",
//     "6",
//     "7",
//     "8",
//     "9",
//     "0",
//     " ",
//     "\n"
// ];

const primes = [
    3,
    7,
    9,
    11,
    13,
    17,
    19,
    21,
    23,
    27,
    29,
    31,
    33,
    37,
    39,
    41,
    43,
    47,
    49,
    51,
    53,
    57,
    59,
    61,
    63,
    67,
    69,
    71,
    73,
    77,
    79,
    81,
    83,
    87,
    89,
    91,
    93,
    97,
    99,
    101,
    103,
    107,
    109,
    111,
    113,
    117,
    119,
    121,
    123,
    127,
    129,
    131,
    133,
    137,
    139,
    141,
    143,
    147,
    149,
    151,
    153,
    157,
    159,
    161,
    163,
    167,
    169,
    171,
    173,
    177,
    179,
    181,
    183,
    187,
    189,
    191,
    193,
    197,
    199,
    201,
    203,
    207,
    209,
    211,
    213,
    217,
    219,
    221,
    223,
    227,
    229,
    231,
    233,
    237,
    239,
    241,
    243,
    247,
    249
];

const loop = (a, m, count) => {
    let num = 0;
    while (num / a < 94) {
        let num = (m * count + 1);
        if (num % a == 0) {
            return (num / a);
        } else {
            count++;
        }
    }
};

const modularInverse = (a, m) => {
    let x = a - m;
    if (-x > a) {
        return loop(a, m, 1);
    } else {
        return loop(x, m, 1) + m;
    }
};

// const checkMod = (a, b) => {
//     if (a % b == 1) {
//         return true;
//     } else if (a % b == 0) {
//         return false;
//     }
//     return checkMod(b, a % b);
// };

// for(i of primes) {
//     console.log(modularInverse(i, 250));
// }

// let num_i = 1;
// let out_li = [];
// while (num_i < 250) {
//     if(checkMod(250, num_i)){
//         out_li.push(num_i);
//     }
//     num_i++;
// }
// console.log(out_li);

document.getElementById('text').focus();

const c_btn = document.querySelector('#copy-btn');
const result = document.querySelector("#result");

const activateBtn = () => {
    c_btn.classList.remove('disable');
};

const deactivateBtn = () => {
    c_btn.classList.add('disable');
};

const textAreaResize = () => {
    result.style.height = '1px';
    result.style.height = (25 + result.scrollHeight) + 'px';
};

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
        if(0 <= x){
            out = out + code(x);
            i = i - (x * 3 ** p);
        }
        p--;
    }
    return (out);
}

console.log(to(128));

const from = (i) => {
    let num = 0;
    let mul = 256;
    for (e of i.split('')) {
        mul /= 4;
        index = getIndex(e, code);
        if (index < 0) {
            return -1;
        }
        num += index * (mul);
    };
    return (num);
}

const encrypt = () => {
    let out = "";
    let msg = document.querySelector('#text').value;
    let msg_li = msg.split('');
    if (msg_li.length > 0) {
        temp_key = primes[(msg_li.length) % primes.length];
        console.log(msg_li.length);
        for (e of msg_li) {
            let num = e.charCodeAt(0);
            if (num < 0) {
                alertMsg("Invalid formats !", "Do not use emojis and this only supports english characters");
                out = "";
                deactivateBtn();
                break;
            } else {
                // num = (num * 3) % key[0];
                num = (num * temp_key) % 250;
                console.log(e, num);
                out += to(num);
                activateBtn();
            }
        };
        result.innerHTML = out;
        textAreaResize();
        c_btn.innerHTML = "copy";
    } else {
        console.error("Enter something first...")
    }
}

const decrypt = () => {
    let out = "";
    let msg = document.querySelector('#text').value;
    let msg_li = msg.match(/.{1,4}/g);
    try {
        temp_key = modularInverse(primes[(msg_li.length) % primes.length], 250);
        for (e of msg_li) {
            if (e.length != 4) {
                alertMsg("Invalid ecrypted format !", "Use only encrypted text that was created from this site");
                out = "";
                deactivateBtn();
                break;
            }
            let num = from(e);
            if (num < 0) {
                alertMsg("Invalid ecrypted format !", "Use only encrypted text that was created from this site");
                out = "";
                deactivateBtn();
                break;
            }
            console.log(num);
            num = (num * temp_key) % 250;
            out += String.fromCharCode(num);
            activateBtn();
        };
        result.innerHTML = out;
        textAreaResize();
        c_btn.innerHTML = "copy";
    } catch (err) {
        console.error("Enter something first...")
    }
}

async function copy() {
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

const clearText = () => {
    document.getElementById('text').focus();
    document.getElementById('text').value = '';
};

addEventListener('resize', () => textAreaResize());






















// const letters = [
//     "A",
//     "B",
//     "C",
//     "D",
//     "E",
//     "F",
//     "G",
//     "H",
//     "I",
//     "J",
//     "K",
//     "L",
//     "M",
//     "N",
//     "O",
//     "P",
//     "Q",
//     "R",
//     "S",
//     "T",
//     "U",
//     "V",
//     "W",
//     "X",
//     "Y",
//     "Z",
//     "a",
//     "b",
//     "c",
//     "d",
//     "e",
//     "f",
//     "g",
//     "h",
//     "i",
//     "j",
//     "k",
//     "l",
//     "m",
//     "n",
//     "o",
//     "p",
//     "q",
//     "r",
//     "s",
//     "t",
//     "u",
//     "v",
//     "w",
//     "x",
//     "y",
//     "z",
//     "~",
//     "!",
//     "@",
//     "#",
//     "$",
//     "%",
//     "^",
//     "&",
//     "*",
//     "(",
//     ")",
//     "-",
//     "_",
//     "+",
//     "=",
//     "{",
//     "[",
//     "]",
//     "}",
//     "|",
//     ";",
//     ":",
//     "'",
//     '"',
//     "<",
//     ">",
//     ",",
//     ".",
//     "?",
//     "/",
//     "1",
//     "2",
//     "3",
//     "4",
//     "5",
//     "6",
//     "7",
//     "8",
//     "9",
//     "0",
//     " ",
//     "\n"
// ];

// const checkDivLen = (mu, n) => {
//     let r = false;
//     for(l of letters) {
//         if((l.charCodeAt(0) * mu) % m < n)
//             r = true;
//         else
//             return false;
//     }
//     return r;
// };

// const checkMod = (a, b) => {
//     if (a % b == 1) {
//         return true;
//     } else if (a % b == 0) {
//         return false;
//     }
//     return checkMod(b, a % b);
// };

// let num_i = 1;
// let count = 0
// let out_li = [];
// while (count < 100) {
//     if (checkMod(m, num_i) && checkDivLen(num_i, 243)) {
//         out_li.push(num_i);
//         count++;
//     }
//     num_i++;
// }
// console.log(out_li);
