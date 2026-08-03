"""Version and authenticated model discovery for the official agy CLI."""

from dataclasses import dataclass
from pathlib import Path

from agy_search.contract import AgyCommandBuilder, ModelSlug
from agy_search.errors import InvalidModelSlugError, OutputInvalidError
from agy_search.models import ModelsResponse, StatusResponse
from agy_search.process import ProcessRequest, ProcessRunner


@dataclass(frozen=True, slots=True)
class AgyDiscovery:
    """Run minimal official discovery commands through a process port."""

    builder: AgyCommandBuilder
    runner: ProcessRunner
    cwd: Path
    timeout_seconds: float

    async def models(self) -> ModelsResponse:
        """Return a non-empty unique allowlist from ``agy models``."""
        output = await self.runner.run(
            ProcessRequest(
                command=self.builder.models_argv(),
                cwd=self.cwd,
                timeout_seconds=self.timeout_seconds,
            ),
        )
        lines = _decode_lines(output.stdout, label="model discovery")
        try:
            slugs = tuple(ModelSlug(line).value for line in lines)
        except InvalidModelSlugError as error:
            raise OutputInvalidError(reason="invalid model discovery output") from error
        if not slugs or len(slugs) != len(set(slugs)):
            raise OutputInvalidError(reason="model discovery must be non-empty and unique")
        return ModelsResponse(models=slugs)

    async def status(self) -> StatusResponse:
        """Prove executable availability plus authenticated model access."""
        output = await self.runner.run(
            ProcessRequest(
                command=self.builder.version_argv(),
                cwd=self.cwd,
                timeout_seconds=self.timeout_seconds,
            ),
        )
        version_lines = _decode_lines(output.stdout, label="version discovery")
        if len(version_lines) != 1:
            raise OutputInvalidError(reason="version discovery must return one line")
        models = await self.models()
        return StatusResponse(
            version=version_lines[0],
            model_count=len(models.models),
        )


def _decode_lines(output: bytes, *, label: str) -> tuple[str, ...]:
    try:
        text = output.decode("utf-8", errors="strict")
    except UnicodeDecodeError as error:
        raise OutputInvalidError(reason=f"{label} was not UTF-8") from error
    return tuple(line.strip() for line in text.splitlines() if line.strip())
