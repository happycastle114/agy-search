"""Validated machine responses for standalone research operations."""

from typing import Annotated, Final, Literal, Self

from pydantic import Field, model_validator
from pydantic_core import PydanticCustomError

from agy_search.models.common import (
    BoundaryModel,
    HttpUrl,
    HttpUrls,
    NonEmptyText,
    require_unique_urls,
)

_UNKNOWN_CITATION_CODE: Final = "unknown_citation"
_UNKNOWN_CITATION_MESSAGE: Final = "finding citations must reference returned sources"


class WebSource(BoundaryModel):
    """One real source consulted during web research."""

    title: NonEmptyText
    url: HttpUrl
    snippet: NonEmptyText
    date: str | None = None
    last_updated: str | None = None


class ExtractPage(BoundaryModel):
    """Content extracted from one explicit web page."""

    url: HttpUrl
    title: NonEmptyText
    content: NonEmptyText


class MapLink(BoundaryModel):
    """One URL discovered while mapping a website."""

    url: HttpUrl
    title: NonEmptyText
    depth: Annotated[int, Field(ge=0)]


class CrawlPage(BoundaryModel):
    """One page read during a bounded crawl."""

    url: HttpUrl
    title: NonEmptyText
    content: NonEmptyText


class ResearchFinding(BoundaryModel):
    """One synthesized finding with source-linked citations."""

    title: NonEmptyText
    summary: NonEmptyText
    citations: HttpUrls


SearchResults = Annotated[tuple[WebSource, ...], Field(min_length=1, max_length=20)]
ExtractResults = Annotated[tuple[ExtractPage, ...], Field(min_length=1, max_length=20)]
MapResults = Annotated[tuple[MapLink, ...], Field(min_length=1, max_length=100)]
CrawlResults = Annotated[tuple[CrawlPage, ...], Field(min_length=1, max_length=50)]
ResearchFindings = Annotated[tuple[ResearchFinding, ...], Field(min_length=1, max_length=20)]


class SearchResponse(BoundaryModel):
    """Source discovery response."""

    object: Literal["search"] = "search"
    results: SearchResults

    @model_validator(mode="after")
    def unique_urls(self) -> Self:
        """Require unique normalized source URLs."""
        require_unique_urls(tuple(result.url for result in self.results))
        return self


class ExtractResponse(BoundaryModel):
    """Explicit page extraction response."""

    object: Literal["extract"] = "extract"
    results: ExtractResults

    @model_validator(mode="after")
    def unique_urls(self) -> Self:
        """Require unique normalized extracted URLs."""
        require_unique_urls(tuple(result.url for result in self.results))
        return self


class MapResponse(BoundaryModel):
    """Website URL map response."""

    object: Literal["map"] = "map"
    base_url: HttpUrl
    results: MapResults

    @model_validator(mode="after")
    def unique_urls(self) -> Self:
        """Require unique normalized discovered URLs."""
        require_unique_urls(tuple(result.url for result in self.results))
        return self


class CrawlResponse(BoundaryModel):
    """Bounded page crawl response."""

    object: Literal["crawl"] = "crawl"
    base_url: HttpUrl
    results: CrawlResults

    @model_validator(mode="after")
    def unique_urls(self) -> Self:
        """Require unique normalized crawled URLs."""
        require_unique_urls(tuple(result.url for result in self.results))
        return self


class ResearchResponse(BoundaryModel):
    """Multi-source synthesis with internally consistent citations."""

    object: Literal["research"] = "research"
    title: NonEmptyText
    summary: NonEmptyText
    findings: ResearchFindings
    sources: SearchResults

    @model_validator(mode="after")
    def citations_reference_sources(self) -> Self:
        """Require unique sources and reject unsupported finding citations."""
        require_unique_urls(tuple(source.url for source in self.sources))
        source_urls = {source.url for source in self.sources}
        citations = {citation for finding in self.findings for citation in finding.citations}
        if not citations.issubset(source_urls):
            raise PydanticCustomError(
                _UNKNOWN_CITATION_CODE,
                _UNKNOWN_CITATION_MESSAGE,
            )
        return self
