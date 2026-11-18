#!/bin/sh
# Gathers the files the site is made of into ./dist, so that only those get
# published (not the Rust source, the tests or the build tools).
#
# This is the whole "build" the host runs. On Cloudflare Pages:
#   Build command:           sh tools/site.sh
#   Build output directory:  dist
#
# Nothing is compiled here. pkg/ is made by ./build.sh on your own machine and
# committed, so the host needs no Rust.
#
# The stylesheet, its font and everything in pkg/ are published inside a folder
# named after their contents (dist/v-<digest>/), and the copy of index.html in
# dist asks for them there. The page and those files only work as a set, but
# browsers are told to keep .css and .js for hours (Cloudflare's Browser Cache
# TTL, which it applies to those file types whatever the site asks for), while
# index.html and the .wasm are fetched fresh every time. Under fixed names a
# returning visitor got the new page with the old stylesheet and the old loader:
# unstyled, and running the slow JavaScript build because the old loader didn't
# fit the new .wasm. A folder whose name changes whenever its contents do can be
# kept for as long as a browser likes.
set -e
cd "$(dirname "$0")/.."

for needed in index.html style_2.css pkg/huh_encrypter.js pkg/huh_encrypter_bg.wasm pkg/fallback/huh_encrypter.js pkg/fallback/huh_encrypter_bg.js pkg/fallback/huh_encrypter_bg.wasm.js; do
    if [ ! -f "$needed" ]; then
        echo "error: $needed is missing. Run ./build.sh and commit pkg/." >&2
        exit 1
    fi
done

# A short digest of what is piped in, with whichever tool this machine has.
digest() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum | cut -c1-12
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 | cut -c1-12
    else
        cksum | tr -s ' \t' '-' | cut -d- -f1,2
    fi
}

# Every versioned file, by name and by content, in an order that never varies.
versioned=$(
    find style_2.css font pkg -type f | LC_ALL=C sort | while IFS= read -r file; do
        printf '%s\n' "$file"
        cat "$file"
    done | digest
)
if [ -z "$versioned" ]; then
    echo "error: could not work out a digest for the site's files." >&2
    exit 1
fi
set="v-$versioned"

rm -rf dist
mkdir -p "dist/$set"
cp -R img dist/
cp -R style_2.css font pkg "dist/$set/"

# index.html, asking for the stylesheet and pkg/ in their folder.
sed -e "s#\./style_2\.css#./$set/style_2.css#g" -e "s#\./pkg/#./$set/pkg/#g" index.html > dist/index.html

# Each of the four must have been found and pointed at the folder; if index.html
# starts naming its files some other way, stop here rather than publish a page
# that asks for files that aren't there.
for asked in "href=\"./$set/style_2.css\"" "'./$set/pkg/huh_encrypter.js'" "'./$set/pkg/huh_encrypter_bg.wasm'" "'./$set/pkg/fallback/huh_encrypter.js'"; do
    if ! grep -q -F -e "$asked" dist/index.html; then
        echo "error: dist/index.html doesn't ask for $asked. Has index.html changed how it names its files?" >&2
        exit 1
    fi
done

echo "site gathered in dist/ (stylesheet and pkg in $set/):"
find dist -type f | sort
