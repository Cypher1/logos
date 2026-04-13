from dataclasses import asdict
from typing import Any

from ollama import Message


def has_all_keys(json_object: dict, keys: list[str]):
    # TODO: Tests
    return all([k in json_object for k in keys])


def has_only_keys(json_object: dict, keys: list[str]):
    # TODO: Tests
    return all([k in keys for k in json_object])


def has_keys(json_object: dict, required: list[str], *, optional: list[str] | None = None):
    # TODO: Tests
    all = required
    if optional is not None:
        all += optional
    return has_all_keys(json_object, required) and has_only_keys(json_object, all)


def from_json(json_object: Any) -> Message | Message.ToolCall | dict:
    # TODO: Tests
    if has_keys(json_object, ["arguments", "name"]):
        name = json_object["name"]
        arguments = json_object["arguments"]
        return Message.ToolCall(function=Message.ToolCall.Function(name=name, arguments=arguments))
    if has_keys(json_object, ["role"], optional=["tool_calls", "tool_name", "content", "thinking", "images"]):
        return Message(
            role = json_object["role"],
            content = json_object.get("content"),
            thinking = json_object.get("thinking"),
            images = json_object.get("images"),
            tool_name = json_object.get("tool_name"),
            tool_calls = json_object.get("tool_calls")
        )
    return json_object


def to_json(value):
    if isinstance(value, (Message.ToolCall, Message)):
        return asdict(value)

    raise TypeError(f"Value {value!r} not serializable")
