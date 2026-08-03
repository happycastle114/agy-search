"""Validated inputs for standalone Antigravity operations."""

from typing import Annotated

from pydantic import Field

from agy_search.models.common import BoundaryModel, HttpUrl, HttpUrls, NonEmptyText

ResultLimit = Annotated[int, Field(ge=1, le=20)]
MapLimit = Annotated[int, Field(ge=1, le=100)]
CrawlLimit = Annotated[int, Field(ge=1, le=50)]
TokenLimit = Annotated[int, Field(gt=0)]
Domains = Annotated[tuple[NonEmptyText, ...], Field(max_length=20)]


class SearchRequest(BoundaryModel):
    """One bounded source-discovery request."""

    query: NonEmptyText
    max_results: ResultLimit = 5
    domains: Domains = ()
    country: NonEmptyText | None = None
    max_tokens_per_page: TokenLimit | None = None


class ExtractRequest(BoundaryModel):
    """One request to read content from explicit web URLs."""

    urls: HttpUrls
    query: NonEmptyText | None = None


class MapRequest(BoundaryModel):
    """One bounded URL-discovery request rooted at a website."""

    url: HttpUrl
    limit: MapLimit = 50
    instructions: NonEmptyText | None = None
    allow_external: bool = False


class CrawlRequest(BoundaryModel):
    """One bounded content crawl rooted at a website."""

    url: HttpUrl
    limit: CrawlLimit = 20
    instructions: NonEmptyText | None = None
    allow_external: bool = False


class ResearchRequest(BoundaryModel):
    """One multi-source synthesis request."""

    query: NonEmptyText
    max_sources: ResultLimit = 10
