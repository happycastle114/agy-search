from agy_search.contract import AgyCommandBuilder, AgyPrintRequest, ModelSlug
from agy_search.enums import AgyEffort


def test_print_command_is_fixed_direct_argv() -> None:
    # Given: one schema-constrained print request
    request = AgyPrintRequest(
        prompt="fixture prompt",
        print_timeout="120s",
        json_schema='{"type":"object"}',
        model=ModelSlug("fixture-model"),
        effort=AgyEffort.HIGH,
    )

    # When: the command builder renders argv
    command = AgyCommandBuilder("/fixture/agy").print_argv(request)

    # Then: plan mode, structured output, model, effort, and prompt are direct arguments
    assert command == (
        "/fixture/agy",
        "--mode",
        "plan",
        "--print-timeout",
        "120s",
        "--output-format",
        "stream-json",
        "--json-schema",
        '{"type":"object"}',
        "--model",
        "fixture-model",
        "--effort",
        "high",
        "-p",
        "fixture prompt",
    )


def test_model_and_version_discovery_commands_are_minimal() -> None:
    # Given: one downstream command builder
    builder = AgyCommandBuilder("agy")

    # When: discovery argv is rendered
    models = builder.models_argv()
    version = builder.version_argv()

    # Then: no print-mode or permission-bypass flags are present
    assert models == ("agy", "models")
    assert version == ("agy", "--version")
