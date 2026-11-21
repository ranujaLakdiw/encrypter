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
# The stylesheet, the fonts, the two scripts in js/ and everything in pkg/ are
# published inside a folder named after their contents (dist/v-<digest>/), and
# the copy of index.html in dist asks for them there. The page and those files
# only work as a set, but browsers are told to keep .css and .js for hours
# (Cloudflare's Browser Cache TTL, which it applies to those file types whatever
# the site asks for), while index.html and the .wasm are fetched fresh every
# time. Under fixed names a returning visitor got the new page with the old
# stylesheet and the old loader: unstyled, and running the slow JavaScript build
# because the old loader didn't fit the new .wasm. A folder whose name changes
# whenever its contents do can be kept for as long as a browser likes.
#
# It also writes dist/_headers, which is where Cloudflare Pages takes the
# response headers from: the page's content policy (the same one index.html
# carries, which is checked) and the rest of what a browser needs to be told.
# See "the headers" below.
set -e
cd "$(dirname "$0")/.."

for needed in index.html style_2.css font/fonts.css js/boot.js js/particles.min.js pkg/huh_encrypter.js pkg/huh_encrypter_bg.wasm pkg/fallback/huh_encrypter.js pkg/fallback/huh_encrypter_bg.js pkg/fallback/huh_encrypter_bg.wasm.js; do
    if [ ! -f "$needed" ]; then
        echo "error: $needed is missing. Run ./build.sh and commit pkg/." >&2
        exit 1
    fi
done

# The stylesheets name their font files by address. One that isn't there (not
# committed, say) would show as nothing worse than the wrong typeface, so look.
missing=$(
    for sheet in style_2.css font/fonts.css; do
        grep -o 'url([^)]*)' "$sheet" | tr -d "'\"" | sed -e 's/^url(//' -e 's/)$//' | while IFS= read -r asked; do
            [ -f "$(dirname "$sheet")/$asked" ] || echo "  $sheet asks for $asked"
        done
    done
)
if [ -n "$missing" ]; then
    echo "error: files the stylesheets ask for are missing:" >&2
    echo "$missing" >&2
    exit 1
fi

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
    find style_2.css font js pkg -type f | LC_ALL=C sort | while IFS= read -r file; do
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
cp -R style_2.css font js pkg "dist/$set/"

# index.html, asking for the stylesheets and the scripts in their folder. (The
# scripts find pkg/ from their own address, so nothing in them needs changing.)
sed -e "s#\./style_2\.css#./$set/style_2.css#g" -e "s#\./font/#./$set/font/#g" -e "s#\./js/#./$set/js/#g" index.html > dist/index.html

# Each of the four must have been found and pointed at the folder; if index.html
# starts naming its files some other way, stop here rather than publish a page
# that asks for files that aren't there.
for asked in "href=\"./$set/style_2.css\"" "href=\"./$set/font/fonts.css\"" "src=\"./$set/js/particles.min.js\"" "src=\"./$set/js/boot.js\""; do
    if ! grep -q -F -e "$asked" dist/index.html; then
        echo "error: dist/index.html doesn't ask for $asked. Has index.html changed how it names its files?" >&2
        exit 1
    fi
done

# The page must load nothing from anywhere but this site: a script from someone
# else's server could read every message and password typed into it. (However
# the tag is written: capitals, single quotes, no quotes at all.)
if grep -i -n -E "(src|href)[[:space:]]*=[[:space:]]*[\"']?[[:space:]]*(https?:)?//" dist/index.html >&2; then
    echo "error: index.html loads the above from another site. Bring the file into the repo instead." >&2
    exit 1
fi

# --- the headers ---------------------------------------------------------------
# The content policy: this site's own files and WebAssembly, and nothing else.
# index.html carries it in a <meta> tag (so the page is held to it wherever it
# is served from) and it is sent again here as a header: a header covers the
# page from its first byte, whatever is added to the markup on the way to the
# browser, and only a header can say frame-ancestors.
#
# The two have to be the same text, to the letter. There are a great many ways
# to loosen a policy and no list of them to check against, so there is no
# checking for bad words: index.html says exactly this, once, or nothing is
# published. To change the policy, change it here and there together.
policy="default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; font-src 'self'; img-src 'self' blob:; connect-src 'self'; base-uri 'none'; form-action 'none'"
if [ "$(grep -c -i 'content-security-policy' index.html)" != "1" ]; then
    echo "error: index.html should mention Content-Security-Policy exactly once: in its one <meta> tag." >&2
    exit 1
fi
in_page=$(sed -n 's/.*<meta http-equiv="Content-Security-Policy" content="\([^"]*\)">.*/\1/p' index.html)
if [ "$in_page" != "$policy" ]; then
    echo "error: the content policy in index.html is not the one tools/site.sh sends." >&2
    echo "  index.html:    $in_page" >&2
    echo "  tools/site.sh: $policy" >&2
    exit 1
fi

cat > dist/_headers <<EOF
# Written by tools/site.sh; change it there.
/*
  Content-Security-Policy: $policy; frame-ancestors 'none'
  Strict-Transport-Security: max-age=31536000; includeSubDomains
  X-Frame-Options: DENY
  X-Content-Type-Options: nosniff
  Referrer-Policy: no-referrer
  Cross-Origin-Opener-Policy: same-origin
  Permissions-Policy: accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()
EOF

echo "site gathered in dist/ (stylesheets, fonts, scripts and pkg in $set/):"
find dist -type f | sort
