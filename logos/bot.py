import asyncio
from collections.abc import Callable
from dataclasses import dataclass, field
from functools import partial
from pathlib import Path
from typing import Any, Awaitable, Literal

from ollama import AsyncClient, ChatResponse, Message
from rich.console import Console, ConsoleOptions, ConsoleRenderable, RenderResult
from rich.markdown import Markdown
from rich.segment import Segment
from rich.syntax import Syntax

from logos.serializers import from_json, to_json
from logos.tools import Sender

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
    sender: Sender
    # TODO: Use the client to pull models as needed.
    client: AsyncClient

    model: str = field(default=DEFAULT_MODEL)
    window_size: int = field(default=DEFAULT_WINDOW_SIZE)
    think: bool | Literal["low", "medium", "high"] = True
    tools: bool = True

    tool_set: dict[str, Callable] = field(default_factory=dict)
    messages: list[Message] = field(default_factory=list)
    user_interrupt: bool = True

    @classmethod
    def skip_fields(cls) -> set[str]:
        return {"sender", "client", "tool_set", "messages"}

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

    async def add_message(self, message: Message) -> None:
        if message.role == "assistant" and message.content:
            # Set up 'direct' chat
            await self.sender.send_nfty_message(message)

        # Also report the debug log as we go.
        await self.sender.send_nfty_thinking(message)
        self.messages.append(message)
        try:
            data = to_json(message)
            with self.state_file.open("a") as f:
                f.write(data + "\n")
                f.flush()
                f.close()
        except FileNotFoundError:
            print(f"State file missing {self.state_file}")

    async def get_response(self) -> ChatResponse:
        # The python client automatically parses functions as a tool schema so we can pass them directly
        # Schemas can be passed directly in the tools list as well
        # TODO: Streaming
        response = await self.client.chat(
            model=self.model,
            messages=self.messages,
            tools=list(self.tool_set.values()) if self.tools else None,
            think=self.think,
        )
        await self.add_message(response.message)
        await self.process_tool_calls(response.message)
        return response

    async def process_tool_call(self, call: Message.ToolCall) -> None:
        if not self.tools:
            return

        # execute the appropriate tool
        tool = self.tool_set.get(call.function.name)
        maybe_result = tool(**call.function.arguments) if tool else "Unknown tool"
        if isinstance(maybe_result, Awaitable):
            result = await maybe_result
        else:
            result = maybe_result
        # add the tool result to the messages
        func = render_function(call.function)
        result = f"{func} = {result!r}"
        await self.add_message(
            Message(role="tool", content=result, tool_name=call.function.name),
        )

    async def process_tool_calls(self, message: Message) -> None:
        if not self.tools:
            return

        if not message.tool_calls:
            return

        calls = {self.process_tool_call(call) for call in message.tool_calls}
        await asyncio.gather(*calls)

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
