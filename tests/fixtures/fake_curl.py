#!/usr/bin/env python3
"""Deterministic curl-compatible redirect and source transport for CLI tests."""

from enum import Enum
import json
import os
import sys
import time
from urllib.parse import urlparse


class CurlFlag(str, Enum):
    SILENT = "--silent"
    SHOW_ERROR = "--show-error"
    FAIL = "--fail"
    LOCATION = "--location"
    TLS_V1_2 = "--tlsv1.2"
    PROTO = "--proto"
    PROTO_REDIR = "--proto-redir"
    MAX_REDIRS = "--max-redirs"
    CONNECT_TIMEOUT = "--connect-timeout"
    MAX_TIME = "--max-time"
    OUTPUT = "--output"
    WRITE_OUT = "--write-out"


class SourceCurlFlag(str, Enum):
    DISABLE = "--disable"
    NO_PROXY = "--noproxy"
    RESOLVE = "--resolve"
    MAX_FILESIZE = "--max-filesize"
    URL = "--url"


class SourcePath(str, Enum):
    SOURCE = "/source"
    PRIMARY = "/primary"
    ALPHA = "/alpha"
    BETA = "/beta"
    RELEASES = "/releases"
    TIMEOUT = "/timeout"
    FAILED = "/failed"
    INVALID = "/invalid"
    LOCAL = "/local"
    LOCAL_UNEXTRACTABLE = "/local-unextractable"
    LOCAL_V25 = "/local-v25"
    LOCAL_AFTER_CUTOFF = "/local-after-cutoff"
    LOCAL_TIE = "/local-tie"
    ANTIGRAVITY_RELEASES = "/antigravity-releases"
    ANTIGRAVITY_AMBIGUOUS = "/antigravity-ambiguous"
    ANTIGRAVITY_MISSING = "/antigravity-missing"
    ANTIGRAVITY_AFTER_CUTOFF = "/antigravity-after-cutoff"


class RedirectFinal(str, Enum):
    ALLOWED = "allowed"
    DISALLOWED = "disallowed"


EXPECTED_PREFIX = [
    CurlFlag.SILENT.value,
    CurlFlag.SHOW_ERROR.value,
    CurlFlag.FAIL.value,
    CurlFlag.LOCATION.value,
    CurlFlag.TLS_V1_2.value,
    CurlFlag.PROTO.value,
    "=https",
    CurlFlag.PROTO_REDIR.value,
    "=https",
    CurlFlag.MAX_REDIRS.value,
    "5",
    CurlFlag.CONNECT_TIMEOUT.value,
    "2",
    CurlFlag.MAX_TIME.value,
    "5",
    CurlFlag.OUTPUT.value,
    "/dev/null",
    CurlFlag.WRITE_OUT.value,
    "%{url_effective}\\n",
]


def main() -> int:
    arguments = sys.argv[1:]
    if arguments and arguments[0] == SourceCurlFlag.DISABLE.value and "--dump-header" in arguments:
        return redirect_main(arguments)
    if arguments and arguments[0] == SourceCurlFlag.DISABLE.value:
        return source_main(arguments)
    if arguments[:-1] != EXPECTED_PREFIX:
        return 64
    source = arguments[-1]
    if source.endswith("/invalid-final"):
        print("file:///private/source")
    elif source.endswith("/multiple-lines"):
        print("https://example.com/first")
        print("https://example.com/second")
    else:
        print("https://example.com/canonical")
    return 0


def redirect_main(arguments: list[str]) -> int:
    try:
        source_url = arguments[arguments.index(SourceCurlFlag.URL.value) + 1]
        final = RedirectFinal(os.environ["AGY_SEARCH_REDIRECT_FINAL"])
    except (ValueError, IndexError, KeyError):
        return 64
    parsed = urlparse(source_url)
    if parsed.hostname == "vertexaisearch.cloud.google.com":
        host = "example.com" if final is RedirectFinal.ALLOWED else "iana.org"
        print("HTTP/1.1 302 Test\r")
        print(f"Location: https://{host}/canonical\r")
        print("\r")
        print("\nAGY_REDIRECT_META:302:0")
    else:
        print("HTTP/1.1 200 Test\r")
        print("\r")
        print("\nAGY_REDIRECT_META:200:0")
    return 0


def source_main(arguments: list[str]) -> int:
    required_pairs = {
        SourceCurlFlag.NO_PROXY: "*",
        SourceCurlFlag.MAX_FILESIZE: str(512 * 1024),
    }
    for flag, expected in required_pairs.items():
        try:
            position = arguments.index(flag.value)
        except ValueError:
            return 64
        if position + 1 >= len(arguments) or arguments[position + 1] != expected:
            return 64
    if CurlFlag.LOCATION.value in arguments or SourceCurlFlag.RESOLVE.value not in arguments:
        return 64
    try:
        url_position = arguments.index(SourceCurlFlag.URL.value)
        source_url = arguments[url_position + 1]
        source_path = SourcePath(urlparse(source_url).path)
    except (ValueError, IndexError):
        return 64
    trace = os.environ.get("AGY_SEARCH_SOURCE_FETCH_TRACE")
    if trace:
        with open(trace, "a", encoding="utf-8") as stream:
            stream.write(json.dumps({"url": source_url, "argv": arguments}) + "\n")
    match source_path:
        case SourcePath.TIMEOUT:
            # The Rust source process owns the deadline and must terminate us.
            time.sleep(5.0)
            return 124
        case SourcePath.FAILED:
            print("source transport failed", file=sys.stderr)
            return 23
        case SourcePath.INVALID:
            print("not a source response")
            return 0
    body = source_body(source_path)
    if body is None:
        print("\nAGY_SOURCE_META:404:0")
        return 22
    print(body, end="")
    print("\nAGY_SOURCE_META:200:0")
    return 0


def source_body(path: SourcePath) -> str | None:
    match path:
        case SourcePath.SOURCE:
            return panels(
                ("newer", "newer fixture", "v2 August 3, 2026"),
                ("older", "older fixture", "v1 August 2, 2026"),
            )
        case SourcePath.PRIMARY:
            return panels(
                ("alpha", "alpha", "alpha-v1 2026-08-03"),
                ("beta", "beta", "beta-v1 2026-08-02"),
            )
        case SourcePath.ALPHA:
            return panels(
                ("alpha", "alpha", "alpha-v2 2026-08-05 alpha-v1 2026-08-02"),
            )
        case SourcePath.BETA:
            return panels(
                ("beta", "beta", "beta-v1 2026-08-04 beta-v2 2026-08-03"),
            )
        case SourcePath.RELEASES:
            return panels(
                ("alpha", "alpha", "alpha-v1 2026-08-02"),
                ("beta", "beta", "beta-v2 2026-08-03"),
            )
        case SourcePath.LOCAL:
            return pinned_panels(
                ("alpha", "alpha", "alpha-v2", "August 5, 2026"),
                ("beta", "beta", "beta-v1", "August 4, 2026"),
            )
        case SourcePath.LOCAL_UNEXTRACTABLE:
            return panels(
                ("alpha", "alpha", "alpha-v2 August 5, 2026"),
                ("beta", "beta", "beta-v1 August 4, 2026"),
            )
        case SourcePath.LOCAL_V25:
            return pinned_panels(
                ("alpha", "alpha", "25.1.0", "August 5, 2026"),
                ("beta", "beta", "24.9.0", "August 4, 2026"),
            )
        case SourcePath.LOCAL_AFTER_CUTOFF:
            return pinned_panels(
                ("alpha", "alpha", "25.2.0", "August 6, 2026"),
                ("beta", "beta", "24.9.0", "August 4, 2026"),
            )
        case SourcePath.LOCAL_TIE:
            return pinned_panels(
                ("alpha", "alpha", "25.1.0", "August 5, 2026"),
                ("beta", "beta", "24.9.0", "August 5, 2026"),
            )
        case SourcePath.ANTIGRAVITY_RELEASES:
            return pinned_panels(
                ("antigravity", "Antigravity 2.0", "2.5.0", "August 4, 2026"),
                ("cli", "Antigravity CLI", "1.1.10", "August 3, 2026"),
            )
        case SourcePath.ANTIGRAVITY_AMBIGUOUS:
            return pinned_panels(
                ("cli-current", "Antigravity CLI", "1.1.10", "August 3, 2026"),
                ("cli-legacy", "Antigravity CLI", "1.1.9", "August 2, 2026"),
            )
        case SourcePath.ANTIGRAVITY_MISSING:
            return pinned_panels(
                ("antigravity", "Antigravity 2.0", "2.5.0", "August 3, 2026"),
            )
        case SourcePath.ANTIGRAVITY_AFTER_CUTOFF:
            return pinned_panels(
                ("cli", "Antigravity CLI", "1.1.11", "August 4, 2026"),
            )
    return None


def panels(*rows: tuple[str, str, str]) -> str:
    tabs = "".join(
        f'<button data-tab="{key}">{label}</button>' for key, label, _ in rows
    )
    bodies = "".join(
        f'<div data-list-panel="{key}"><div data-section-row>{row}</div></div>'
        for key, _, row in rows
    )
    return f"<div data-tab-buttons></div>{tabs}{bodies}"


def pinned_panels(*rows: tuple[str, str, str, str]) -> str:
    tabs = "".join(
        f'<button data-tab="{key}">{label}</button>'
        for key, label, _, _ in rows
    )
    bodies = "".join(
        f'<div data-list-panel="{key}"><div data-section-row>'
        f'<span data-date-pin>{value} {date}</span>'
        f'</div></div>'
        for key, _, value, date in rows
    )
    return f"<div data-tab-buttons></div>{tabs}{bodies}"


if __name__ == "__main__":
    raise SystemExit(main())
