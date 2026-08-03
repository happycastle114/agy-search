from agy_search.enums import CliCommand, CliExitCode


def test_command_set_matches_the_standalone_cli_contract() -> None:
    # Given: the public command enum
    # When: its serialized values are enumerated
    values = {command.value for command in CliCommand}

    # Then: every supported standalone operation is present exactly once
    assert values == {
        "status",
        "models",
        "search",
        "extract",
        "map",
        "crawl",
        "research",
    }


def test_exit_codes_are_stable_and_distinct() -> None:
    # Given: the public exit enum
    # When: its integer values are enumerated
    values = [int(code) for code in CliExitCode]

    # Then: the documented machine exit classes remain stable and unique
    assert values == [0, 2, 3, 4, 5, 6, 7]
    assert len(values) == len(set(values))
