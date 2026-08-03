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
