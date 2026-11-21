// Starts the page.
//
// The app runs as WebAssembly where the browser can do that, and as the very
// same program translated to plain JavaScript (../pkg/fallback) where it can't:
// WebAssembly switched off (iOS Lockdown Mode, some locked-down setups) or a
// browser too old for the module.
//
// This used to sit inside index.html. It is a file of its own so that the page
// can refuse every inline script (see the Content-Security-Policy in
// index.html): whatever else ends up in the page's markup then cannot run.

// Fetch every font file now, whether or not anything on screen needs it yet.
// The fonts come in pieces, one per script (Latin, Cyrillic, Vietnamese…), and a
// browser left to itself fetches a piece the first time a letter from it is
// drawn. That would make what is typed, or what a huH unlocks to, decide which
// files the page asks its server for: a hint about the text, to anyone watching
// the requests. Asked for up front, they say nothing.
try {
    document.fonts.forEach((face) => {
        face.load().catch(() => {});
    });
} catch (_) {
    // No font loading here: the browser falls back to fetching as it draws.
}

const canRunWebAssembly = () => {
    try {
        // Where WebAssembly is blocked by policy the object is still there and
        // compiling is what throws, so try it on the empty module.
        const empty = new WebAssembly.Module(Uint8Array.of(0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00));
        return new WebAssembly.Instance(empty) instanceof WebAssembly.Instance;
    } catch (_) {
        return false;
    }
};

// Importing the JavaScript build is what starts it.
const startAsJavaScript = () => import('../pkg/fallback/huh_encrypter.js');

// Adding "?js" to the address runs the JavaScript build anywhere, to try it out.
const askedForJavaScript = new URLSearchParams(location.search).has('js');

if (!askedForJavaScript && canRunWebAssembly()) {
    // Fetch the module while its loader is being fetched. (Its address is worked
    // out from this file's own, as an import's is, not from the page's.)
    const hint = document.createElement('link');
    hint.rel = 'preload';
    hint.as = 'fetch';
    hint.type = 'application/wasm';
    hint.crossOrigin = 'anonymous';
    hint.href = new URL('../pkg/huh_encrypter_bg.wasm', import.meta.url).href;
    document.head.append(hint);

    import('../pkg/huh_encrypter.js')
        .then(({ default: start }) => start())
        .catch((error) => {
            // A browser that refuses the module does so before any of the app
            // has run, so the JavaScript build can still take over cleanly.
            if (error instanceof WebAssembly.CompileError || error instanceof WebAssembly.LinkError) {
                return startAsJavaScript();
            }
            throw error;
        });
} else {
    startAsJavaScript();
}
