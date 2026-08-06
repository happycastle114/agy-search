"""Machine-consumed custom-agent contract for fake content runs."""

from pathlib import Path


def valid_agent_invocation(arguments: list[str]) -> bool:
    """Return whether one invocation selects the isolated search-only agent."""
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
        "  - search_web",
        "  - read_url_content",
        "  - view_file",
        "  - grep_search",
        "mainAgent: true",
        "subagent: false",
        "inheritMcp: false",
    }
    return (
        arguments.count("--agent") == 1
        and agent_name == "agy-search"
        and required_structure.issubset(definition.splitlines())
        and "call_mcp_tool" not in definition
    )
