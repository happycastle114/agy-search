"""Shared strict boundary types for web research operations."""

from typing import Annotated, ClassVar, Final

from pydantic import (
    AfterValidator,
    AnyHttpUrl,
    BaseModel,
    ConfigDict,
    Field,
    StringConstraints,
    TypeAdapter,
)
from pydantic_core import PydanticCustomError

NonEmptyText = Annotated[str, StringConstraints(strip_whitespace=True, min_length=1)]
_HTTP_URL_ADAPTER: Final = TypeAdapter(AnyHttpUrl)
_DUPLICATE_URL_CODE: Final = "duplicate_urls"
_DUPLICATE_URL_MESSAGE: Final = "response URLs must be unique"


def normalize_http_url(value: str) -> str:
    """Parse and normalize one HTTP(S) URL while preserving a string type."""
    return str(_HTTP_URL_ADAPTER.validate_python(value))


HttpUrl = Annotated[
    str,
    StringConstraints(strip_whitespace=True, min_length=1),
    AfterValidator(normalize_http_url),
]
HttpUrls = Annotated[tuple[HttpUrl, ...], Field(min_length=1, max_length=20)]


class BoundaryModel(BaseModel):
    """Immutable, closed model for every CLI trust boundary."""

    model_config: ClassVar[ConfigDict] = ConfigDict(frozen=True, extra="forbid")


def require_unique_urls(urls: tuple[str, ...]) -> None:
    """Reject duplicate normalized URLs at response boundaries."""
    if len(urls) != len(set(urls)):
        raise PydanticCustomError(_DUPLICATE_URL_CODE, _DUPLICATE_URL_MESSAGE)
