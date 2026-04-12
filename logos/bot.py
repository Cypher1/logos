from collections.abc import Callable
from dataclasses import dataclass, field
from json import decoder, dump, load
from pathlib import Path
from typing import Literal

from ollama import ChatResponse, Message, chat
from rich.console import (Console, ConsoleOptions, ConsoleRenderable,
                          RenderResult)
from rich.markdown import Markdown
from rich.segment import Segment
from rich.syntax import Syntax


@dataclass
class IndentedRenderable:
    renderable: ConsoleRenderable | str
    indent: int

    def __rich_console__(
        self, console: Console, options: ConsoleOptions
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
    model: str
    messages: list[Message] = field(default_factory=list)
    tools: dict[str, Callable] = field(default_factory=dict)
    think: bool | Literal["low", "medium", "high"] = True

    def add_tool(self, func) -> "Bot":
        self.tools[func.__name__] = func
        return self

    def load_state(self, state_file: Path) -> None:
        try:
            print("Loading...")
            with open(state_file, "r") as f:
                data = load(f)
            for obj in data:
                self.messages.append(Message(**obj))
        except FileNotFoundError:
            # No previous state
            return
        except decoder.JSONDecodeError:
            # Start fresh
            return

    def save_state(self, state_file: Path) -> None:
        print("Saving...")
        data = [dict(message) for message in self.messages]
        with open(state_file, "w") as f:
            dump(data, f)
        print("Done")

    def add_message(
        self, role: str, content: str, tool_name: str | None = None
    ) -> "Bot":
        self.messages.append(Message(role=role, content=content, tool_name=tool_name))
        return self

    def get_response(self) -> ChatResponse:
        # The python client automatically parses functions as a tool schema so we can pass them directly
        # Schemas can be passed directly in the tools list as well
        response = chat(
            model=self.model,
            messages=self.messages,
            tools=list(self.tools.values()),
            think=self.think,
        )
        self.messages.append(response.message)
        self.process_tool_calls(response.message)
        return response

    def process_tool_calls(self, message) -> None:
        if not message.tool_calls:
            return

        for call in message.tool_calls:
            # execute the appropriate tool
            # TODO: Async
            tool = self.tools.get(call.function.name)
            if tool:
                result = tool(**call.function.arguments)
            else:
                result = "Unknown tool"
            # add the tool result to the messages
            func = render_function(call.function)
            result = f"{func} = {result!r}"
            self.add_message("tool", result, tool_name=call.function.name)

    def render_message(self, console, message: Message):
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
        if message.images:
            console.print(message.images, style="red")
