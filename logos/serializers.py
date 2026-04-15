from json import dumps, loads
from typing import Any

from ollama import Message
from pydantic import ValidationError


def has_all_keys(json_object: dict, keys: list[str]):
    # TODO: Tests
    return all([k in json_object for k in keys])


def has_only_keys(json_object: dict, keys: list[str]):
    # TODO: Tests
    return all([k in keys for k in json_object])


def has_keys(
    json_object: dict, required: list[str], *, optional: list[str] | None = None
):
    # TODO: Tests
    all = required
    if optional is not None:
        all += optional
    return has_all_keys(json_object, required) and has_only_keys(json_object, all)


def object_hook(
    json_object: Any,
) -> Message | Message.ToolCall | Message.ToolCall.Function | dict:
    # TODO: Tests
    if has_keys(json_object, ["arguments", "name"]):
        return Message.ToolCall.Function.model_validate(json_object)
    if has_keys(json_object, ["function"]):
        return Message.ToolCall.model_validate(json_object)
    if has_keys(
        json_object,
        ["role"],
        optional=["tool_calls", "tool_name", "content", "thinking", "images"],
    ):
        return Message.model_validate(json_object)
    return json_object


def from_json(
    json_object: str,
) -> Message | Message.ToolCall | Message.ToolCall.Function | dict:
    try:
        return loads(json_object, object_hook=object_hook)
    except ValidationError as e:
        print(json_object)
        for err in e.errors():
            print(err)
        raise e


def pre_process(value) -> dict:
    if isinstance(value, Message):
        return value.model_dump(mode="json")
    if isinstance(value, Message.ToolCall):
        return value.model_dump(mode="json")
    if isinstance(value, Message.ToolCall.Function):
        return value.model_dump(mode="json")
    raise TypeError(f"Value {value!r} not serializable")


def to_json(value):
    return dumps(value, default=pre_process)
