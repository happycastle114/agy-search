"""Strict parsing of source-backed Antigravity structured event streams."""

from typing import ClassVar

from pydantic import BaseModel, ConfigDict, Field, ValidationError

from agy_search.enums import AgyEventName, AgyResearchTool, AgyStepState, AgyStepType
from agy_search.errors import OutputInvalidError
from agy_search.models.common import BoundaryModel

type JsonScalar = str | int | float | bool | None
type JsonValue = JsonScalar | list[JsonValue] | dict[str, JsonValue]
type JsonObject = dict[str, JsonValue]


class AgyUsage(BoundaryModel):
    """Safe token telemetry from one completed print-mode run."""

    input_tokens: int = Field(default=0, ge=0)
    output_tokens: int = Field(default=0, ge=0)
    thinking_tokens: int = Field(default=0, ge=0)
    cache_read_tokens: int = Field(default=0, ge=0)
    total_tokens: int = Field(default=0, ge=0)


class _ToolInfo(BaseModel):
    model_config: ClassVar[ConfigDict] = ConfigDict(frozen=True, extra="ignore")

    name: str


class _StepUpdate(BaseModel):
    model_config: ClassVar[ConfigDict] = ConfigDict(frozen=True, extra="ignore")

    step_type: str
    state: str | None = None
    tool_info: _ToolInfo | None = None


class _Result(BaseModel):
    model_config: ClassVar[ConfigDict] = ConfigDict(frozen=True, extra="ignore")

    structured_output: JsonObject | None = None
    usage: AgyUsage = AgyUsage()
    duration_seconds: float = Field(default=0.0, ge=0)
    num_turns: int = Field(default=0, ge=0)


class _Event(BaseModel):
    model_config: ClassVar[ConfigDict] = ConfigDict(frozen=True, extra="ignore")

    event: str
    step_update: _StepUpdate | None = None
    result: _Result | None = None


class StructuredRun[ResponseT: BoundaryModel](BoundaryModel):
    """One validated response with evidence and safe telemetry."""

    response: ResponseT
    research_tools: tuple[AgyResearchTool, ...]
    usage: AgyUsage
    duration_seconds: float = Field(ge=0)
    num_turns: int = Field(ge=0)


def parse_structured_run[ResponseT: BoundaryModel](
    output: str,
    response_type: type[ResponseT],
    *,
    required_tools: frozenset[AgyResearchTool],
) -> StructuredRun[ResponseT]:
    """Validate a terminal response and require matching live-tool evidence."""
    events = _parse_events(output)
    research_tools = _collect_research_tools(events)
    if required_tools.isdisjoint(research_tools):
        raise OutputInvalidError(reason="required research tool did not execute")
    result = _terminal_result(events)
    if result.structured_output is None:
        raise OutputInvalidError(reason="missing structured output")
    try:
        response = response_type.model_validate(result.structured_output)
    except ValidationError as error:
        raise OutputInvalidError(reason="structured output failed validation") from error
    return StructuredRun[ResponseT](
        response=response,
        research_tools=research_tools,
        usage=result.usage,
        duration_seconds=result.duration_seconds,
        num_turns=result.num_turns,
    )


def _parse_events(output: str) -> tuple[_Event, ...]:
    events: list[_Event] = []
    for line in output.splitlines():
        candidate = line.strip()
        if not candidate:
            continue
        try:
            events.append(_Event.model_validate_json(candidate))
        except ValidationError as error:
            raise OutputInvalidError(reason="malformed event stream") from error
    if not events:
        raise OutputInvalidError(reason="empty event stream")
    return tuple(events)


def _collect_research_tools(events: tuple[_Event, ...]) -> tuple[AgyResearchTool, ...]:
    tools: list[AgyResearchTool] = []
    for event in events:
        if _event_name(event.event) is not AgyEventName.STEP_UPDATE:
            continue
        step = event.step_update
        if (
            step is None
            or _step_type(step.step_type) is not AgyStepType.TOOL
            or _step_state(step.state) is not AgyStepState.DONE
        ):
            continue
        if step.tool_info is None:
            continue
        tool = _research_tool(step.tool_info.name)
        if tool is not None and tool not in tools:
            tools.append(tool)
    return tuple(tools)


def _terminal_result(events: tuple[_Event, ...]) -> _Result:
    for event in reversed(events):
        if _event_name(event.event) is AgyEventName.RESULT:
            if event.result is None:
                raise OutputInvalidError(reason="malformed result event")
            return event.result
    raise OutputInvalidError(reason="missing result event")


def _event_name(value: str) -> AgyEventName | None:
    try:
        return AgyEventName(value)
    except ValueError:
        return None


def _step_type(value: str) -> AgyStepType | None:
    try:
        return AgyStepType(value)
    except ValueError:
        return None


def _step_state(value: str | None) -> AgyStepState | None:
    try:
        return AgyStepState(value) if value is not None else None
    except ValueError:
        return None


def _research_tool(value: str) -> AgyResearchTool | None:
    try:
        return AgyResearchTool(value)
    except ValueError:
        return None
