"""Canonical JSON rendering and atomic context-saving output."""

import json
import os
import tempfile
from pathlib import Path

from agy_search.errors import OutputWriteError
from agy_search.models.common import BoundaryModel


def render_json(response: BoundaryModel) -> str:
    """Render one deterministic UTF-8 JSON document."""
    return json.dumps(
        response.model_dump(mode="json"),
        ensure_ascii=False,
        indent=2,
        sort_keys=True,
    )


def write_json_file(response: BoundaryModel, output_path: Path) -> None:
    """Atomically replace one explicit output file with canonical JSON."""
    document = f"{render_json(response)}\n"
    temporary_path: Path | None = None
    try:
        output_path.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            dir=output_path.parent,
            prefix=f".{output_path.name}.",
            suffix=".tmp",
            delete=False,
        ) as temporary:
            _ = temporary.write(document)
            temporary.flush()
            os.fsync(temporary.fileno())
            temporary_path = Path(temporary.name)
        _ = temporary_path.replace(output_path)
    except OSError as error:
        if temporary_path is not None:
            temporary_path.unlink(missing_ok=True)
        raise OutputWriteError(reason=type(error).__name__) from error
