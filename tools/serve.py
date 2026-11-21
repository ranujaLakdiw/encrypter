#!/usr/bin/env python3
"""Serves ./dist on this machine with the response headers in dist/_headers.

A plain file server sends none of those headers, so a page that works from one
can still break once the host starts sending its content policy. This is for
trying the gathered site the way the host will serve it, before publishing:

    sh tools/site.sh && python3 tools/serve.py [port]

Only what tools/site.sh writes is understood: one rule, for every path ("/*").
"""
import functools
import http.server
import sys
from pathlib import Path

DIST = Path(__file__).resolve().parent.parent / "dist"


def headers_for_every_path(path: Path) -> list[tuple[str, str]]:
    found, applies = [], False
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if not line[0].isspace():
            applies = line.strip() == "/*"
        elif applies and ":" in line:
            name, value = line.split(":", 1)
            found.append((name.strip(), value.strip()))
    return found


class Handler(http.server.SimpleHTTPRequestHandler):
    extra: list[tuple[str, str]] = []

    def end_headers(self):
        for name, value in self.extra:
            self.send_header(name, value)
        # Nothing kept between runs: what is on screen is what is in dist/ now.
        self.send_header("Cache-Control", "no-store")
        super().end_headers()


def main():
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8769
    if not (DIST / "_headers").is_file():
        sys.exit("dist/_headers is missing: run `sh tools/site.sh` first.")
    Handler.extra = headers_for_every_path(DIST / "_headers")
    if not any(name.lower() == "content-security-policy" for name, _ in Handler.extra):
        sys.exit("dist/_headers has no Content-Security-Policy for /*.")
    handler = functools.partial(Handler, directory=str(DIST))
    with http.server.ThreadingHTTPServer(("127.0.0.1", port), handler) as server:
        print(f"dist/ on http://localhost:{port} with {len(Handler.extra)} headers from dist/_headers")
        server.serve_forever()


if __name__ == "__main__":
    main()
