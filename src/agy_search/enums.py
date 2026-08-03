"""Closed public and downstream command variants."""

from enum import IntEnum, StrEnum, unique


@unique
class CliCommand(StrEnum):
    """Public standalone operations."""

    STATUS = "status"
    MODELS = "models"
    SEARCH = "search"
    EXTRACT = "extract"
    MAP = "map"
    CRAWL = "crawl"
    RESEARCH = "research"


@unique
class CliExitCode(IntEnum):
    """Stable process exit classes."""

    SUCCESS = 0
    USAGE = 2
    UNAVAILABLE = 3
    TIMEOUT = 4
    UPSTREAM = 5
    OUTPUT = 6
    MODEL = 7


@unique
class AgyRunMode(StrEnum):
    """Supported non-interactive Antigravity execution modes."""

    PLAN = "plan"


@unique
class AgyOutputFormat(StrEnum):
    """Machine-readable Antigravity output formats used by this CLI."""

    STREAM_JSON = "stream-json"


@unique
class AgyEffort(StrEnum):
    """Official Antigravity reasoning-effort values."""

    LOW = "low"
    MEDIUM = "medium"
    HIGH = "high"


@unique
class AgyEventName(StrEnum):
    """Antigravity event variants consumed at the trust boundary."""

    INIT = "init"
    STEP_UPDATE = "step_update"
    RESULT = "result"


@unique
class AgyStepType(StrEnum):
    """Step variants relevant to source-evidence collection."""

    TOOL = "tool"


@unique
class AgyStepState(StrEnum):
    """Terminal step states accepted as completed tool evidence."""

    DONE = "DONE"


@unique
class AgyResearchTool(StrEnum):
    """Antigravity tools that can provide live web evidence."""

    SEARCH_WEB = "search_web"
    READ_URL_CONTENT = "read_url_content"


@unique
class ContentOperation(StrEnum):
    """Schema-constrained content operations delegated to Antigravity."""

    SEARCH = "search"
    EXTRACT = "extract"
    MAP = "map"
    CRAWL = "crawl"
    RESEARCH = "research"
