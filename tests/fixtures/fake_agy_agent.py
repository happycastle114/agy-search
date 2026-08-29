"""Machine-consumed custom-agent contract for fake content runs."""

from pathlib import Path


def valid_agent_invocation(arguments: list[str]) -> bool:
    """Return whether one invocation selects a least-privilege isolated agent."""
    try:
        agent_index = arguments.index("--agent")
        agent_name = arguments[agent_index + 1]
        definition = Path(
            ".agents/agents/agy-search/agent.md"
        ).read_text(encoding="utf-8")
    except (ValueError, IndexError, OSError):
        return False
    required_structure = {
        "name: agy-search",
        "mainAgent: true",
        "subagent: false",
        "inheritMcp: false",
    }
    configured_tools = {
        line for line in definition.splitlines() if line.startswith("  - ")
    }
    allowed_tools = {"  - search_web"}
    has_explicit_no_tools = "tools: []" in definition.splitlines()
    return (
        arguments.count("--agent") == 1
        and arguments.count("--dangerously-skip-permissions") == 1
        and agent_name == "agy-search"
        and required_structure.issubset(definition.splitlines())
        and (configured_tools or has_explicit_no_tools)
        and configured_tools.issubset(allowed_tools)
        and "call_mcp_tool" not in definition
        and "  - view_file" not in definition
        and "  - grep_search" not in definition
    )
