import pytest
from pydantic import ValidationError

from agy_search.models import (
    CrawlPage,
    CrawlResponse,
    ExtractPage,
    ExtractResponse,
    MapLink,
    MapRequest,
    MapResponse,
    ResearchFinding,
    ResearchRequest,
    ResearchResponse,
    SearchRequest,
    SearchResponse,
    WebSource,
)


def _source(url: str = "https://example.com/source") -> WebSource:
    return WebSource(title="Source", url=url, snippet="Source-backed summary")


def test_operation_responses_have_distinct_machine_discriminators() -> None:
    # Given: one valid response for every content operation
    responses = (
        SearchResponse(results=(_source(),)),
        ExtractResponse(
            results=(
                ExtractPage(
                    url="https://example.com/page",
                    title="Page",
                    content="Extracted content",
                ),
            ),
        ),
        MapResponse(
            base_url="https://example.com",
            results=(MapLink(url="https://example.com/docs", title="Docs", depth=1),),
        ),
        CrawlResponse(
            base_url="https://example.com",
            results=(
                CrawlPage(
                    url="https://example.com/docs",
                    title="Docs",
                    content="Crawled content",
                ),
            ),
        ),
        ResearchResponse(
            title="Research",
            summary="Source-backed synthesis",
            findings=(
                ResearchFinding(
                    title="Finding",
                    summary="Finding detail",
                    citations=("https://example.com/source",),
                ),
            ),
            sources=(_source(),),
        ),
    )

    # When: their discriminators are serialized
    objects = tuple(response.object for response in responses)

    # Then: callers can exhaustively distinguish every response shape
    assert objects == ("search", "extract", "map", "crawl", "research")


def test_operation_requests_are_frozen() -> None:
    # Given: one parsed request boundary
    request = SearchRequest(query="query", max_results=1)

    # When: a caller attempts to mutate it
    with pytest.raises(ValidationError):
        request.max_results = 2

    # Then: Pydantic rejects mutation at the boundary


def test_non_http_source_is_rejected() -> None:
    # Given: a source with a non-web URL
    # When: the boundary parses it
    with pytest.raises(ValidationError):
        _ = _source("file:///private/source")

    # Then: local and synthetic URL schemes cannot enter a response


def test_duplicate_map_urls_are_rejected() -> None:
    # Given: duplicate discovered URLs
    duplicate = MapLink(url="https://example.com/docs", title="Docs", depth=1)

    # When: a map response parses the duplicates
    with pytest.raises(ValidationError):
        _ = MapResponse(
            base_url="https://example.com",
            results=(duplicate, duplicate),
        )

    # Then: a caller never receives duplicate map entries


def test_research_citations_must_reference_returned_sources() -> None:
    # Given: a finding citing a URL absent from the source list
    finding = ResearchFinding(
        title="Finding",
        summary="Finding detail",
        citations=("https://missing.example/source",),
    )

    # When: the research response crosses the boundary
    with pytest.raises(ValidationError):
        _ = ResearchResponse(
            title="Research",
            summary="Synthesis",
            findings=(finding,),
            sources=(_source(),),
        )

    # Then: unsupported citations fail closed


def test_search_limit_is_bounded() -> None:
    # Given: a search request beyond its public work bound
    # When: the request boundary parses it
    with pytest.raises(ValidationError):
        _ = SearchRequest(query="query", max_results=21)

    # Then: unbounded work is rejected before Antigravity runs


def test_map_limit_is_bounded() -> None:
    # Given: a map request beyond its public work bound
    # When: the request boundary parses it
    with pytest.raises(ValidationError):
        _ = MapRequest(url="https://example.com", limit=101)

    # Then: unbounded work is rejected before Antigravity runs


def test_research_limit_is_bounded() -> None:
    # Given: a research request beyond its public work bound
    # When: the request boundary parses it
    with pytest.raises(ValidationError):
        _ = ResearchRequest(query="query", max_sources=21)

    # Then: unbounded work is rejected before Antigravity runs
