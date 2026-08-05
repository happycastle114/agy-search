#!/usr/bin/env python3
"""Antigravity fixture with an explicit, traceable version preflight."""

import os
import subprocess
import sys
import time

import fake_agy


def append_trace(event: str) -> None:
    """Append one invocation event when the test requests a trace."""
    trace_path = os.environ.get("AGY_SEARCH_VERSION_TRACE")
    if trace_path is None:
        return
    descriptor = os.open(trace_path, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
    try:
        os.write(descriptor, f"{event}\n".encode())
    finally:
        os.close(descriptor)


def sleep_for(variable: str) -> None:
    """Sleep for the test-configured delay, if one is present."""
    configured = os.environ.get(variable)
    if configured is not None:
        time.sleep(float(configured))


def spawn_background_child() -> None:
    """Create a same-process-group child and record its PID for cleanup proof."""
    pid_path = os.environ.get("AGY_SEARCH_VERSION_CHILD_PID")
    if pid_path is None:
        return
    child = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(30)"])
    with open(pid_path, "w", encoding="utf-8") as handle:
        handle.write(str(child.pid))


def version() -> int:
    """Emit the configured official CLI version payload."""
    append_trace("version")
    spawn_background_child()
    sleep_for("AGY_SEARCH_VERSION_DELAY")
    value = os.environ.get("AGY_SEARCH_VERSION", "1.1.10")
    if value:
        print(value)
    return 0


def models() -> int:
    """Emit deterministic models without executing a content prompt."""
    append_trace("models")
    sleep_for("AGY_SEARCH_MODELS_DELAY")
    print("fixture-model")
    print("fixture-model-high")
    return 0


def main() -> int:
    """Dispatch only the narrow Antigravity compatibility surface."""
    if sys.argv[1:] == ["--version"]:
        return version()
    if sys.argv[1:] == ["models"]:
        return models()
    append_trace("content")
    return fake_agy.main()


if __name__ == "__main__":
    raise SystemExit(main())
