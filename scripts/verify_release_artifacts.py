"""Verify that built distributions retain every advertised runtime asset."""

import json
import sys
import tarfile
import zipfile
from pathlib import Path

_SKILL_FILES = (
    "SKILL.md",
    "agents/openai.yaml",
    "references/commands.md",
)
_DIST_ARGUMENT_COUNT = 2


def _single_match(dist_directory: Path, pattern: str) -> Path:
    matches = tuple(dist_directory.glob(pattern))
    if len(matches) != 1:
        message = f"expected exactly one {pattern} artifact, found {len(matches)}"
        raise ValueError(message)
    return matches[0]


def _require_members(members: set[str], expected: tuple[str, ...], artifact: Path) -> None:
    missing = tuple(member for member in expected if member not in members)
    if missing:
        message = f"{artifact.name} is missing: {', '.join(missing)}"
        raise ValueError(message)


def verify(dist_directory: Path) -> dict[str, object]:
    """Check the wheel and sdist skill payloads and return a JSON-ready report."""
    wheel = _single_match(dist_directory, "*.whl")
    source = _single_match(dist_directory, "*.tar.gz")

    with zipfile.ZipFile(wheel) as archive:
        wheel_members = set(archive.namelist())
    wheel_skill_members = tuple(f"agy_search/skills/agy-search/{name}" for name in _SKILL_FILES)
    _require_members(wheel_members, wheel_skill_members, wheel)

    with tarfile.open(source, mode="r:gz") as archive:
        source_members = set(archive.getnames())
    source_skill_members = tuple(
        next(member for member in source_members if member.endswith(f"/skills/agy-search/{name}"))
        for name in _SKILL_FILES
    )
    _require_members(source_members, source_skill_members, source)

    return {
        "object": "distribution-verification",
        "skill_files": list(_SKILL_FILES),
        "sdist": source.name,
        "wheel": wheel.name,
    }


def main() -> None:
    """Verify the requested distribution directory and print canonical JSON."""
    dist_directory = Path(sys.argv[1]) if len(sys.argv) == _DIST_ARGUMENT_COUNT else Path("dist")
    try:
        report = verify(dist_directory)
    except (OSError, ValueError, StopIteration) as error:
        _ = sys.stderr.write(f"error: {error}\n")
        raise SystemExit(1) from error
    _ = sys.stdout.write(f"{json.dumps(report, sort_keys=True, separators=(',', ':'))}\n")


if __name__ == "__main__":
    main()
