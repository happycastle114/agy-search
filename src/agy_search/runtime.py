"""Application composition and exhaustive content-operation dispatch."""

from dataclasses import dataclass
from pathlib import Path
from typing import final

from agy_search.backend import AgyBackend, AgyBackendConfig
from agy_search.contract import AgyCommandBuilder, ModelSlug
from agy_search.discovery import AgyDiscovery
from agy_search.enums import AgyEffort
from agy_search.errors import UnknownModelError
from agy_search.models import (
    CrawlRequest,
    ExtractRequest,
    MapRequest,
    ModelsResponse,
    ResearchRequest,
    SearchRequest,
    StatusResponse,
)
from agy_search.models.common import BoundaryModel
from agy_search.process import AnyioProcessRunner

type ContentRequest = SearchRequest | ExtractRequest | MapRequest | CrawlRequest | ResearchRequest


@dataclass(frozen=True, slots=True)
class RuntimeOptions:
    """User-selected process and model options."""

    agy_path: str
    model: ModelSlug | None
    effort: AgyEffort | None
    timeout_seconds: float
    isolation_base: Path


@final
class CliRuntime:
    """Composed discovery and content services for one CLI invocation."""

    def __init__(self, options: RuntimeOptions) -> None:
        """Compose the official command contract with one bounded runner."""
        builder = AgyCommandBuilder(options.agy_path)
        runner = AnyioProcessRunner(max_concurrency=1)
        self._selected_model = options.model
        self._discovery = AgyDiscovery(
            builder=builder,
            runner=runner,
            cwd=Path.cwd(),
            timeout_seconds=min(options.timeout_seconds, 30.0),
        )
        self._backend = AgyBackend(
            builder=builder,
            runner=runner,
            config=AgyBackendConfig(
                isolation_base=options.isolation_base,
                timeout_seconds=options.timeout_seconds,
                print_timeout=f"{options.timeout_seconds:g}s",
                model=options.model,
                effort=options.effort,
            ),
        )

    async def status(self) -> StatusResponse:
        """Return readiness proven against live discovery."""
        return await self._discovery.status()

    async def models(self) -> ModelsResponse:
        """Return dynamically discovered model slugs."""
        return await self._discovery.models()

    async def content(self, request: ContentRequest) -> BoundaryModel:
        """Validate model selection and exhaustively dispatch a content request."""
        await self._validate_selected_model()
        match request:
            case SearchRequest():
                return await self._backend.search(request)
            case ExtractRequest():
                return await self._backend.extract(request)
            case MapRequest():
                return await self._backend.map(request)
            case CrawlRequest():
                return await self._backend.crawl(request)
            case ResearchRequest():
                return await self._backend.research(request)

    async def _validate_selected_model(self) -> None:
        if self._selected_model is None:
            return
        models = await self._discovery.models()
        if self._selected_model.value not in models.models:
            raise UnknownModelError(model=self._selected_model.value)
