from collections.abc import Callable
from dataclasses import dataclass, field
from functools import partial
from pathlib import Path
from typing import Any, Literal

from ollama import ChatResponse, Message, chat
from rich.console import Console, ConsoleOptions, ConsoleRenderable, RenderResult
from rich.markdown import Markdown
from rich.segment import Segment
from rich.syntax import Syntax

from logos import tools
from logos.serializers import from_json, to_json

DEFAULT_MODEL = "gemma4:latest"
DEFAULT_WINDOW_SIZE = 5


@dataclass
class IndentedRenderable:
    renderable: ConsoleRenderable | str
    indent: int

    def __rich_console__(
        self,
        console: Console,
        options: ConsoleOptions,
    ) -> RenderResult:
        segments = console.render(self.renderable, options)
        lines = Segment.split_lines(segments)
        for line in lines:
            yield Segment(" " * self.indent)
            yield from line
            yield Segment("\n")


def render_function(function: Message.ToolCall.Function) -> str:
    args = ", ".join(f"{k}={v!r}" for k, v in function.arguments.items())
    return f"{function.name}({args})"


@dataclass
class Bot:
    state_file: Path
    model: str = field(default=DEFAULT_MODEL)
    window_size: int = field(default=DEFAULT_WINDOW_SIZE)
    think: bool | Literal["low", "medium", "high"] = True
    tools: bool = True

    tool_set: dict[str, Callable] = field(default_factory=dict)
    messages: list[Message] = field(default_factory=list)

    @classmethod
    def skip_fields(cls) -> set[str]:
        return {"tool_set", "messages"}

    def set(self, key, value: str):
        if key in Bot.skip_fields():
            raise ValueError(key)
        if key == "think":
            value = value.lower()
            if value == "true":
                setattr(self, key, True)
            elif value == "false":
                setattr(self, key, False)
            else:
                setattr(self, key, value)
        elif type(self.get(key)) is int:
            setattr(self, key, int(value))
        elif type(self.get(key)) is Path:
            setattr(self, key, Path(value))
        else:
            setattr(self, key, value)

    def get(self, key) -> Any:
        if key in Bot.skip_fields():
            raise ValueError(key)
        return getattr(self, key)

    def add_tool(self, func, instance=None, namespace=None) -> "Bot":
        name = f"{func.__name__}"
        if instance:
            impl = partial(func, instance)
            if namespace:
                name = f"{namespace}_{name}"
            # For some reason 'wraps' and 'partial' don't handle this properly.
            # from functools import WRAPPER_ASSIGNMENTS
            impl.__annotations__ = func.__annotations__
            impl.__doc__ = func.__doc__
            impl.__dict__["__name__"] = name
            func = impl
        if name in self.tool_set:
            # TODO: Handle this?
            raise Exception(f" Bot already has tool for name '{name}'")
        self.tool_set[name] = func
        return self

    def load_state(self) -> None:
        try:
            print("Loading...")
            with self.state_file.open() as f:
                # TODO: Only load the last N messages
                for line in f:
                    data = from_json(line)
                    if not isinstance(data, Message):
                        raise Exception(f"No parse: {line}")
                    self.messages.append(data)
        except FileNotFoundError:
            # No previous state
            return

    def save_state(self) -> None:
        print("Saving...")
        data = [to_json(message) for message in self.messages]
        with self.state_file.open("w") as f:
            f.writelines(data)
            f.flush()
            f.close()
        print("Done")

    def add_message(self, message: Message) -> None:
        if message.role == "assistant" and not message.thinking and message.content:
            # Set up 'direct' chat
            tools.send_nfty_message(message.content)

        # Also report the debug log as we go.
        tools.send_nfty_thinking(message.model_dump_json())
        self.messages.append(message)
        try:
            data = to_json(message)
            with self.state_file.open("a") as f:
                f.write(data + "\n")
                f.flush()
                f.close()
        except FileNotFoundError:
            print(f"State file missing {self.state_file}")

    def get_response(self) -> ChatResponse:
        # The python client automatically parses functions as a tool schema so we can pass them directly
        # Schemas can be passed directly in the tools list as well
        response = chat(
            model=self.model,
            messages=self.messages,
            tools=list(self.tool_set.values()) if self.tools else None,
            think=self.think,
        )
        self.add_message(response.message)
        if self.tools:
            self.process_tool_calls(response.message)
        return response

    def process_tool_calls(self, message: Message) -> None:
        if not self.tools:
            return

        if not message.tool_calls:
            return

        for call in message.tool_calls:
            # execute the appropriate tool
            # TODO: Async
            tool = self.tool_set.get(call.function.name)
            result = tool(**call.function.arguments) if tool else "Unknown tool"
            # add the tool result to the messages
            func = render_function(call.function)
            result = f"{func} = {result!r}"
            self.add_message(
                Message(role="tool", content=result, tool_name=call.function.name),
            )

    def render_message(self, console: Console, message: Message):
        out: ConsoleRenderable | str
        if message.tool_name and message.content:
            out = Syntax(message.content, "python", theme="monokai")
            out = IndentedRenderable(out, 1)
            console.print(out, style="red")
        else:
            if message.thinking:
                console.print(message.role, style="red")
                out = message.thinking
                out = IndentedRenderable(out, 1)
                console.print(out, style="yellow")
            if message.content:
                console.print(message.role, style="red")
                out = Markdown(message.content)
                out = IndentedRenderable(out, 1)
                console.print(out, style="green")
            if message.tool_calls:
                out = "tool_calls"
                console.print(out, style="red")
                out = "\n".join("\t" + render_function(call.function) for call in message.tool_calls)
                out = IndentedRenderable(out, 1)
                console.print(out, style="yellow")
        if message.images:
            console.print(message.images, style="red")
