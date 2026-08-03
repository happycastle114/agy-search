"""Standalone Typer surface for source-backed Antigravity research."""

import sys
import tempfile
from pathlib import Path
from typing import Annotated

import anyio
import typer
from pydantic import ValidationError

from agy_search import __version__
from agy_search.contract import ModelSlug
from agy_search.enums import AgyEffort, CliCommand
from agy_search.errors import (
    AgySearchError,
    InvalidCommandError,
    InvalidModelSlugError,
)
from agy_search.failure import exit_code_for_error
from agy_search.models import (
    CrawlRequest,
    ExtractRequest,
    MapRequest,
    ResearchRequest,
    SearchRequest,
)
from agy_search.models.common import BoundaryModel
from agy_search.output import render_json, write_json_file
from agy_search.runtime import CliRuntime, ContentRequest, RuntimeOptions

_STDIN_LIMIT_BYTES = 100 * 1024
_MODEL_PARAMETER_MESSAGE = "model must be one non-empty slug"
_REQUEST_PARAMETER_MESSAGE = "request values failed validation"
_STDIN_PARAMETER_MESSAGE = "stdin must be non-empty and at most 100 KiB"

type RequestValue = str | int | bool | None | tuple[str, ...]

app = typer.Typer(
    help="Source-backed web research through the Google Antigravity CLI.",
    invoke_without_command=True,
    no_args_is_help=True,
    pretty_exceptions_enable=False,
)


@app.callback()
def configure(
    context: typer.Context,
    agy_path: Annotated[str, typer.Option(envvar="AGY_SEARCH_AGY_PATH")] = "agy",
    model: Annotated[str | None, typer.Option()] = None,
    effort: Annotated[AgyEffort | None, typer.Option()] = None,
    timeout: Annotated[float, typer.Option(min=1.0, max=1800.0)] = 120.0,
    version: Annotated[bool, typer.Option("--version", is_eager=True)] = False,
) -> None:
    """Configure the downstream executable, model, effort, and deadline."""
    if version:
        typer.echo(__version__)
        raise typer.Exit
    try:
        selected_model = ModelSlug(model) if model is not None else None
    except InvalidModelSlugError as error:
        raise typer.BadParameter(_MODEL_PARAMETER_MESSAGE) from error
    context.obj = RuntimeOptions(
        agy_path=_resolve_executable(agy_path),
        model=selected_model,
        effort=effort,
        timeout_seconds=timeout,
        isolation_base=Path(tempfile.gettempdir()) / "agy-search",
    )


@app.command()
def status(
    context: typer.Context,
    output: Annotated[Path | None, typer.Option("-o", "--output")] = None,
) -> None:
    """Check agy availability, version, authentication, and model discovery."""
    _invoke(context, command=CliCommand.STATUS, request=None, output=output)


@app.command()
def models(
    context: typer.Context,
    output: Annotated[Path | None, typer.Option("-o", "--output")] = None,
) -> None:
    """List model slugs dynamically reported by the current agy account."""
    _invoke(context, command=CliCommand.MODELS, request=None, output=output)


@app.command()
def search(
    context: typer.Context,
    query: Annotated[str, typer.Argument(help="Query text, or - to read stdin.")],
    max_results: Annotated[int, typer.Option("--max-results", "-n", min=1, max=20)] = 5,
    domain: Annotated[list[str] | None, typer.Option("--domain")] = None,
    country: Annotated[str | None, typer.Option()] = None,
    max_tokens_per_page: Annotated[int | None, typer.Option(min=1)] = None,
    output: Annotated[Path | None, typer.Option("-o", "--output")] = None,
) -> None:
    """Discover live web sources for a query."""
    request = _request(
        SearchRequest,
        query=_query_text(query),
        max_results=max_results,
        domains=tuple(domain or ()),
        country=country,
        max_tokens_per_page=max_tokens_per_page,
    )
    _invoke(context, command=None, request=request, output=output)


@app.command()
def extract(
    context: typer.Context,
    urls: Annotated[list[str], typer.Argument(help="One or more explicit HTTP(S) URLs.")],
    query: Annotated[str | None, typer.Option()] = None,
    output: Annotated[Path | None, typer.Option("-o", "--output")] = None,
) -> None:
    """Extract content from explicit web pages."""
    request = _request(ExtractRequest, urls=tuple(urls), query=query)
    _invoke(context, command=None, request=request, output=output)


@app.command("map")
def map_command(
    context: typer.Context,
    url: Annotated[str, typer.Argument()],
    limit: Annotated[int, typer.Option(min=1, max=100)] = 50,
    instructions: Annotated[str | None, typer.Option()] = None,
    allow_external: Annotated[bool, typer.Option()] = False,
    output: Annotated[Path | None, typer.Option("-o", "--output")] = None,
) -> None:
    """Discover a bounded same-origin website URL map."""
    request = _request(
        MapRequest,
        url=url,
        limit=limit,
        instructions=instructions,
        allow_external=allow_external,
    )
    _invoke(context, command=None, request=request, output=output)


@app.command()
def crawl(
    context: typer.Context,
    url: Annotated[str, typer.Argument()],
    limit: Annotated[int, typer.Option(min=1, max=50)] = 20,
    instructions: Annotated[str | None, typer.Option()] = None,
    allow_external: Annotated[bool, typer.Option()] = False,
    output: Annotated[Path | None, typer.Option("-o", "--output")] = None,
) -> None:
    """Read a bounded set of same-origin website pages."""
    request = _request(
        CrawlRequest,
        url=url,
        limit=limit,
        instructions=instructions,
        allow_external=allow_external,
    )
    _invoke(context, command=None, request=request, output=output)


@app.command()
def research(
    context: typer.Context,
    query: Annotated[str, typer.Argument(help="Question text, or - to read stdin.")],
    max_sources: Annotated[int, typer.Option(min=1, max=20)] = 10,
    output: Annotated[Path | None, typer.Option("-o", "--output")] = None,
) -> None:
    """Produce a multi-source synthesis with validated citations."""
    request = _request(
        ResearchRequest,
        query=_query_text(query),
        max_sources=max_sources,
    )
    _invoke(context, command=None, request=request, output=output)


def _query_text(value: str) -> str:
    if value != "-":
        return value
    text = sys.stdin.read(_STDIN_LIMIT_BYTES + 1)
    normalized = text.strip()
    if not normalized or len(text.encode("utf-8")) > _STDIN_LIMIT_BYTES:
        raise typer.BadParameter(_STDIN_PARAMETER_MESSAGE)
    return normalized


def _resolve_executable(value: str) -> str:
    candidate = Path(value).expanduser()
    is_path = candidate.is_absolute() or candidate.parent != Path()
    return str(candidate.resolve()) if is_path else value


def _request[RequestT: BoundaryModel](
    request_type: type[RequestT],
    **values: RequestValue,
) -> RequestT:
    try:
        return request_type.model_validate(values)
    except ValidationError as error:
        raise typer.BadParameter(_REQUEST_PARAMETER_MESSAGE) from error


def _invoke(
    context: typer.Context,
    *,
    command: CliCommand | None,
    request: ContentRequest | None,
    output: Path | None,
) -> None:
    options = context.ensure_object(RuntimeOptions)
    try:
        response = anyio.run(_run, options, command, request)
        if output is None:
            typer.echo(render_json(response))
        else:
            write_json_file(response, output)
            typer.echo(f"wrote {output}", err=True)
    except AgySearchError as error:
        typer.echo(f"error: {error}", err=True)
        raise typer.Exit(code=int(exit_code_for_error(error))) from error


async def _run(
    options: RuntimeOptions,
    command: CliCommand | None,
    request: ContentRequest | None,
) -> BoundaryModel:
    runtime = CliRuntime(options)
    match command:
        case CliCommand.STATUS:
            return await runtime.status()
        case CliCommand.MODELS:
            return await runtime.models()
        case _:
            pass
    if request is None:
        raise InvalidCommandError(reason="content command requires a request")
    return await runtime.content(request)


def main() -> None:
    """Run the installed console entry point."""
    app()
