// Page comparison harness (run from the browser console or an automation tool).
//
// Drives a page through the app's main states and snapshots, for every element:
// tag, id, classes, attributes, own text, layout rectangle and a few computed
// styles. Run it on two builds of the page (same viewport) and diff the results;
// that is how the Rust page was checked against the JavaScript one it replaced.
//
//   const S = await HuhCompare.run(await (await fetch('./tests/vectors.json')).json());
//   localStorage.setItem('snap:a', JSON.stringify(S));      // then again on the other build
//   HuhCompare.diff(JSON.parse(localStorage['snap:a']), JSON.parse(localStorage['snap:b']));

window.HuhCompare = (() => {
    const $ = (id) => document.getElementById(id);
    const wait = (ms = 150) => new Promise((r) => setTimeout(r, ms));

    // A hidden or background tab never gets animation frames; timers stand in.
    const useTimersForFrames = () => {
        window.requestAnimationFrame = (cb) => setTimeout(() => cb(performance.now()), 0);
    };

    // Animations are frozen mid-flight in a background tab (and would smear any
    // measurement anyway), so they are switched off while measuring.
    const holdStill = () => {
        const style = document.createElement('style');
        style.textContent = '*, *::before, *::after { animation: none !important; transition: none !important; }';
        document.head.appendChild(style);
    };

    // Capture what "Copy" buttons copy instead of touching the real clipboard.
    const captureClipboard = () => {
        window.__copied = [];
        Object.defineProperty(navigator.clipboard, 'writeText', {
            configurable: true,
            value: async (t) => { window.__copied.push(String(t)); },
        });
    };

    const click = async (id, ms) => { $(id).click(); await wait(ms); };

    // Locking and unlocking take a moment (each step is shown before the next), and
    // both big buttons wait while they run. This waits them out.
    const idle = async () => {
        await wait(40);
        while ($('encrypt-btn').disabled || $('decrypt-btn').disabled) await wait(20);
        await wait(80);
    };
    const lock = async () => { $('encrypt-btn').click(); await idle(); };
    const unlock = async () => { $('decrypt-btn').click(); await idle(); };
    const type = async (id, value, ms) => {
        const el = $(id);
        el.value = value;
        el.dispatchEvent(new Event('input', { bubbles: true }));
        await wait(ms);
    };
    const attach = async (file) => {
        const dt = new DataTransfer();
        dt.items.add(file);
        const input = $('file-input');
        input.files = dt.files;
        input.dispatchEvent(new Event('change', { bubbles: true }));
        await wait(300);
    };

    const SKIP_TAGS = new Set(['SCRIPT', 'NOSCRIPT', 'CANVAS']);
    const SKIP_ATTRS = new Set(['class', 'id', 'style', 'data-what']);
    const SKIP_CLASSES = new Set(['pop', 'shake', 'bounce-anim']);

    // `skip` lists ids to leave out (with everything inside them), for comparing
    // against a build that doesn't have those elements yet.
    const snap = (skip = []) => {
        const out = [];
        const walk = (el, depth) => {
            if (SKIP_TAGS.has(el.tagName.toUpperCase()) || skip.includes(el.id)) return;
            const attrs = [...el.attributes]
                .filter((a) => !SKIP_ATTRS.has(a.name))
                .map((a) => `${a.name}=${a.value.replace(/^blob:.*/, 'blob:')}`)
                .sort();
            const text = [...el.childNodes]
                .filter((n) => n.nodeType === 3)
                .map((n) => n.textContent.replace(/\s+/g, ' ').trim())
                .filter(Boolean)
                .join('|');
            const r = el.getBoundingClientRect();
            const cs = getComputedStyle(el);
            const classes = [...el.classList].filter((c) => !SKIP_CLASSES.has(c)).sort();
            out.push({
                key: '  '.repeat(depth) + el.tagName.toLowerCase() + (el.id ? `#${el.id}` : '') + classes.map((c) => `.${c}`).join(''),
                attrs: attrs.join(' '),
                text,
                rect: [r.x, r.y, r.width, r.height].map((v) => Math.round(v * 2) / 2).join(','),
                css: [cs.display, cs.color, cs.backgroundColor, cs.fontSize, cs.fontFamily.split(',')[0]].join(' / '),
                value: typeof el.value === 'string' ? el.value.length : undefined,
            });
            if (el.id === 'particles-js') return;
            for (const c of el.children) walk(c, depth + 1);
        };
        for (const c of document.body.children) walk(c, 0);
        return out;
    };

    const PNG = Uint8Array.from(atob('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg=='), (c) => c.charCodeAt(0));

    // `vec` is tests/vectors.json: messages and a file locked by the original JavaScript page.
    const run = async (vec, { skip = [] } = {}) => {
        useTimersForFrames();
        holdStill();
        captureClipboard();
        const S = {};
        const shot = (name) => { S[name] = snap(skip); };

        shot('initial');
        await type('text', 'hello, internet'); shot('typed');
        await type('text', 'huHUhuHUhuHUhuH'); shot('huh-typed');
        await click('mode-session'); shot('session-mode');
        await click('mode-password');
        await type('text', ''); await lock(); shot('error-empty');
        await type('text', 'hello'); await lock(); shot('error-no-password');

        await type('pass', vec.text[1].pass); await type('text', vec.text[1].huh);
        await unlock(); shot('unlocked-text');
        await type('pass', 'wrong'); await unlock(); shot('wrong-password');
        await type('pass', vec.text[4].pass); await type('text', vec.text[4].huh);
        await unlock(); shot('unlocked-long');
        await type('pass', vec.files[0].pass); await type('text', vec.files[0].huh);
        await unlock(); shot('unlocked-file');
        await click('clear'); shot('cleared');

        await attach(new File(['just some notes\n'], 'notes.txt', { type: 'text/plain' })); shot('attached-file');
        await lock(); shot('locked-file');
        await click('attach-remove');
        await attach(new File([PNG], 'dot.png', { type: 'image/png' })); shot('attached-image');
        await click('attach-remove');
        await attach(new File([vec.text[0].huh], 'note.huh', { type: '' })); shot('attached-huh');
        await type('pass', vec.text[0].pass); await unlock(); shot('unlocked-from-file');
        await click('clear');
        await type('text', 'hi'); await lock(); shot('locked-text');
        await click('copy-btn', 300); shot('copied');

        await click('mode-session'); await click('start-pairing', 300); shot('dialog');
        await type('session-other-pk', vec.kem.pk, 900); shot('dialog-reply-ready');
        await click('session-start-btn'); shot('dialog-missing-code');
        await click('session-cancel-btn', 400); shot('dialog-closed');
        return S;
    };

    // Lists every difference between two runs. `loose` names elements whose text
    // legitimately differs from run to run (random salts, session names).
    const diff = (a, b, loose = ['result-meta', 'file-size', 'file-sample', 'session-name', 'result']) => {
        const out = [];
        for (const state of Object.keys(a)) {
            const [x, y] = [a[state], b[state] || []];
            if (x.length !== y.length) out.push(`${state}: ${x.length} elements vs ${y.length}`);
            for (let i = 0; i < Math.min(x.length, y.length); i++) {
                const isLoose = loose.some((id) => x[i].key.trim().includes(`#${id}`));
                for (const field of ['key', 'attrs', 'text', 'rect', 'css', 'value']) {
                    if (isLoose && (field === 'text' || field === 'value')) continue;
                    if (x[i][field] !== y[i][field]) {
                        out.push(`${state} · ${x[i].key.trim()} · ${field}: ${JSON.stringify(x[i][field])} vs ${JSON.stringify(y[i][field])}`);
                    }
                }
            }
        }
        return out;
    };

    return { run, diff, snap, click, type, attach, wait, idle, lock, unlock, useTimersForFrames, captureClipboard };
})();
