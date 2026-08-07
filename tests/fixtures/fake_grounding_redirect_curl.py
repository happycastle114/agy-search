#!/usr/bin/env python3
import json
import os
import sys
from urllib.parse import urlparse

args = sys.argv[1:]
mode = os.environ["AGY_REDIRECT_MODE"]
required_pairs = {
    "--noproxy": "*",
    "--proxy": "",
    "--cookie": "",
    "--netrc-file": "/dev/null",
    "--hsts": "",
    "--alt-svc": "",
    "--max-redirs": "0",
    "--proto": "=https",
}
if not args or args[0] != "--disable" or "--location" in args:
    raise SystemExit(64)
for flag, value in required_pairs.items():
    try:
        position = args.index(flag)
    except ValueError:
        raise SystemExit(64)
    if position + 1 >= len(args) or args[position + 1] != value:
        raise SystemExit(64)
try:
    url = args[args.index("--url") + 1]
    resolve = args[args.index("--resolve") + 1]
except (ValueError, IndexError):
    raise SystemExit(64)
host = urlparse(url).hostname
if host is None or not resolve.startswith(f"{host}:443:"):
    raise SystemExit(64)
initial = "/grounding-api-redirect/" in urlparse(url).path
with open(os.environ["AGY_REDIRECT_TRACE"], "a", encoding="utf-8") as stream:
    stream.write(json.dumps({"url": url, "argv": args}) + "\n")

if mode == "large-final-body" and not initial and host != "vertexaisearch.cloud.google.com":
    if "--head" not in args or "--max-filesize" in args:
        print("Maximum file size exceeded", file=sys.stderr)
        raise SystemExit(63)
if mode == "transport-failure":
    raise SystemExit(63)

if mode == "success":
    if initial:
        location = "../continue?token=real-style"
        status = 302
    elif host == "vertexaisearch.cloud.google.com":
        location = "https://example.com/canonical?source=grounding"
        status = 307
    else:
        location = None
        status = 200
elif mode == "google-wrapper":
    if host in {"google.com", "www.google.com"}:
        location, status = "https://example.com/canonical", 302
    else:
        location, status = None, 200
elif mode == "trailing-dot":
    if initial:
        location, status = "https://example.com/canonical", 302
    else:
        location, status = None, 200
elif mode == "same-transport-origin":
    if initial:
        location, status = (
            "https://vertexaisearch.cloud.google.com/another-transport-path",
            302,
        )
    else:
        location, status = None, 200
elif mode in {"two-origins", "two-origins-one-dead", "retry-projection"}:
    if initial:
        token = urlparse(url).path.rsplit("/", maxsplit=1)[-1]
        if token == "primary":
            location = "https://example.com/primary"
        elif token == "secondary":
            location = "https://iana.org/secondary"
        else:
            location = f"https://example.com/{token}"
        status = 302
    elif mode in {"two-origins-one-dead", "retry-projection"} and (
        host == "iana.org" or urlparse(url).path.startswith("/dead-")
    ):
        location = None
        status = 404
    else:
        location = None
        status = 200
elif mode == "direct-validation":
    path = urlparse(url).path
    if path == "/redirecting":
        location, status = "https://iana.org/terminal", 302
    elif path.startswith("/dead-"):
        location, status = None, 404
    else:
        location, status = None, 200
elif mode == "restricted-disallowed":
    if initial:
        location, status = "https://iana.org/out-of-scope", 302
    else:
        location, status = None, 200
elif mode == "private-v4":
    location, status = "https://127.0.0.1/secret", 302
elif mode == "private-v6":
    location, status = "https://[::1]/secret", 302
elif mode == "link-local-v4":
    location, status = "https://169.254.169.254/latest/meta-data", 302
elif mode == "link-local-v6":
    location, status = "https://[fe80::1]/secret", 302
elif mode == "multicast-v4":
    location, status = "https://224.0.0.1/secret", 302
elif mode == "multicast-v6":
    location, status = "https://[ff02::1]/secret", 302
elif mode == "documentation-v4":
    location, status = "https://192.0.2.1/secret", 302
elif mode == "documentation-v6":
    location, status = "https://[2001:db8::1]/secret", 302
elif mode == "reserved-v4":
    location, status = "https://240.0.0.1/secret", 302
elif mode == "special-v4":
    location, status = "https://192.88.99.1/secret", 302
elif mode == "six-to-four-v6":
    location, status = "https://[2002:808:808::1]/secret", 302
elif mode == "future-documentation-v6":
    location, status = "https://[3fff::1]/secret", 302
elif mode == "userinfo":
    location, status = "https://user@example.com/secret", 302
elif mode == "port":
    location, status = "https://example.com:8443/secret", 302
elif mode == "fragment":
    location, status = "https://example.com/secret#internal", 302
elif mode == "http":
    location, status = "http://example.com/secret", 302
elif mode == "loop":
    location, status = url, 302
elif mode == "too-many-hops":
    hop = int(urlparse(url).query.removeprefix("hop=") or "0")
    location, status = f"https://example.com/continue?hop={hop + 1}", 302
elif mode == "oversize-header":
    sys.stdout.write("HTTP/1.1 302 Found\r\nX-Fill: " + ("a" * 70000) + "\r\n")
    sys.stdout.write("Location: https://example.com/final\r\n\r\n")
    sys.stdout.write("\nAGY_REDIRECT_META:302:0\n")
    raise SystemExit(0)
elif mode == "malformed-location":
    sys.stdout.write("HTTP/1.1 302 Found\r\nLocation: https://[::1\r\n\r\n")
    sys.stdout.write("\nAGY_REDIRECT_META:302:0\n")
    raise SystemExit(0)
elif mode == "malformed-status":
    sys.stdout.write("NOT-HTTP 302 Found\r\nLocation: https://example.com/final\r\n\r\n")
    sys.stdout.write("\nAGY_REDIRECT_META:302:0\n")
    raise SystemExit(0)
elif mode == "multiple-location":
    sys.stdout.write("HTTP/1.1 302 Found\r\n")
    sys.stdout.write("Location: https://example.com/first\r\n")
    sys.stdout.write("Location: https://example.com/second\r\n\r\n")
    sys.stdout.write("\nAGY_REDIRECT_META:302:0\n")
    raise SystemExit(0)
elif mode == "redirect-without-location":
    location, status = None, 302
elif mode == "success-with-location":
    location, status = "https://example.com/unexpected", 200
elif mode == "large-final-body":
    if initial:
        location, status = "../continue?token=real-style", 302
    elif host == "vertexaisearch.cloud.google.com":
        location, status = "https://example.com/canonical?source=grounding", 307
    else:
        location, status = None, 200
elif mode == "head-rejected-terminal":
    if initial:
        location, status = "https://example.com/head-rejected", 302
    elif "--head" in args:
        location, status = None, 404
    else:
        if "--range" not in args or "--max-filesize" not in args:
            raise SystemExit(64)
        location, status = None, 200
elif mode == "head-rejected-disallowed-redirect":
    if initial:
        location, status = "https://example.com/head-rejected", 302
    elif "--head" in args:
        location, status = None, 404
    else:
        if "--range" not in args or "--max-filesize" not in args:
            raise SystemExit(64)
        location, status = "https://iana.org/out-of-scope", 302
else:
    raise SystemExit(64)

sys.stdout.write(f"HTTP/1.1 {status} Test\r\n")
if location is not None:
    sys.stdout.write(f"Location: {location}\r\n")
if mode == "large-final-body" and not initial and host != "vertexaisearch.cloud.google.com":
    sys.stdout.write("Content-Length: 1860000\r\n")
sys.stdout.write("\r\n")
sys.stdout.write(f"\nAGY_REDIRECT_META:{status}:0\n")
