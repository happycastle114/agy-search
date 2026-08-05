"""Shared typed values for deterministic search fixture scenarios."""

from collections.abc import Callable
from enum import Enum

JsonValue = None | bool | int | float | str | list["JsonValue"] | dict[str, "JsonValue"]
Emitter = Callable[..., None]


class SourcePolicy(str, Enum):
    PRIMARY_FIRST = "primary_first"


class ScopePolicy(str, Enum):
    COMPLETE_REQUESTED_SCOPE = "complete_requested_scope"


class DatePolicy(str, Enum):
    EXPLICIT_SOURCE_ONLY = "explicit_source_only"


class VerificationMode(str, Enum):
    TEMPORAL_COMPARISON = "temporal_comparison"


class Effort(str, Enum):
    LOW = "low"


class SingleScopeQuery(str, Enum):
    COMPLETE = "temporal-single-scope"
    AMBIGUOUS = "temporal-single-scope-ambiguous"
    MISSING = "temporal-single-scope-missing"
    AFTER_CUTOFF = "temporal-single-scope-after-cutoff"
