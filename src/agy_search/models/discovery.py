"""Public machine responses for downstream discovery commands."""

from typing import Annotated, Literal

from pydantic import Field

from agy_search.models.common import BoundaryModel, NonEmptyText

ModelSlugs = Annotated[tuple[NonEmptyText, ...], Field(min_length=1)]


class ModelsResponse(BoundaryModel):
    """Dynamically discovered Antigravity model slugs."""

    object: Literal["models"] = "models"
    models: ModelSlugs


class StatusResponse(BoundaryModel):
    """Ready state proven by version and authenticated model discovery."""

    object: Literal["status"] = "status"
    available: Literal[True] = True
    version: NonEmptyText
    model_count: Annotated[int, Field(gt=0)]
