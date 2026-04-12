import sys
from json import dump, load, decoder
from pathlib import Path
from prompt_toolkit import PromptSession
from collections.abc import Callable
from dataclasses import dataclass, field
from typing import Literal

from ollama import ChatResponse, Message, chat
from rich.console import Console
from rich.markdown import Markdown
from rich.syntax import Syntax
from rich.console import ConsoleRenderable, ConsoleOptions, RenderResult
from rich.segment import Segment

MODEL='gemma4:latest'


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
    args = ", ".join(f"{k}={v!r}" for k,v in function.arguments.items())
    return f"{function.name}({args})"


def get_temperature(city: str) -> str:
  """Get the current temperature for a city

  Args:
    city: The name of the city

  Returns:
    The current temperature for the city
  """
  temperatures = {
    "New York": "22°C",
    "London": "15°C",
    "Tokyo": "18°C"
  }
  return temperatures.get(city, "Unknown")

def get_conditions(city: str) -> str:
  """Get the current weather conditions for a city

  Args:
    city: The name of the city

  Returns:
    The current weather conditions for the city
  """
  conditions = {
    "New York": "Partly cloudy",
    "London": "Rainy",
    "Tokyo": "Sunny"
  }
  return conditions.get(city, "Unknown")

@dataclass
class Bot:
    model: str
    messages: list[Message] = field(default_factory=list)
    tools: dict[str, Callable] = field(default_factory=dict)
    think: bool | Literal['low', 'medium', 'high'] = True

    def add_tool(self, func) -> 'Bot':
        self.tools[func.__name__] = func
        return self

    def load_state(self, state_file: Path) -> None:
        try:
            print("Loading...")
            with open(state_file, 'r') as f:
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
        with open(state_file, 'w') as f:
            dump(data, f)
        print("Done")

    def add_message(self, role: str, content: str, tool_name: str | None = None) -> 'Bot':
        self.messages.append(Message(role=role, content= content, tool_name=tool_name))
        return self

    def get_response(self) -> ChatResponse:
        # The python client automatically parses functions as a tool schema so we can pass them directly
        # Schemas can be passed directly in the tools list as well
        response = chat(model=self.model, messages=self.messages, tools=list(self.tools.values()), think=self.think)
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
                result = 'Unknown tool'
            # add the tool result to the messages
            func = render_function(call.function)
            result = f"{func} = {result!r}"
            self.add_message('tool', result, tool_name = call.function.name)

    def render_message(self, console, message: Message):
        if message.tool_name and message.content:
            out = Syntax(message.content, "python", theme="monokai")
            out = IndentedRenderable(out, 1)
            console.print(out, style='red')
        else:
            if message.thinking:
                console.print(message.role, style='red')
                out = message.thinking
                out = IndentedRenderable(out, 1)
                console.print(out, style='yellow')
            if message.content:
                console.print(message.role, style='red')
                out = Markdown(message.content)
                out = IndentedRenderable(out, 1)
                console.print(out, style='green')
        if message.images:
            console.print(message.images, style='red')


def main():
    console = Console()
    session = PromptSession()

    assistant = Bot(MODEL) \
        .add_tool(get_temperature) \
        .add_tool(get_conditions)

    # Load in the previous session
    state_file = Path.home() / ".logos.jsonl"
    assistant.load_state(state_file)

    while True:
        try:
            console.clear()
            for message in assistant.messages:
                assistant.render_message(console, message)

            last = assistant.messages[-1] if assistant.messages else None
            if last is None or (last.role != 'user' and last.role != 'tool' and last.content):
                console.print('user', style='red')
                response = session.prompt('> ')
                if response:
                    assistant.add_message('user', response)
            else:
                assistant.get_response()
        except KeyboardInterrupt:
            break
        except EOFError:
            break
        except Exception as e:
            print(e, file=sys.stderr)
            # Then quit

    assistant.save_state(state_file)


if __name__ == '__main__':
    main()


test_prompt = """
Please adopt the persona of Logos, my research assistant at our Programming Language theory
research group. This is our first interaction.
Please never make up things that aren't in our chat or discoverable via the tools provided.
We work on a little language called Tako, originally made by me, Eleanor
Pratt. Let's get to work
"""

next_prompt = """
Awesome, what tools would you like to help work on it? I have a local check out of tako for
you. It's built in Rust. I also have a large set of personal notes that I'd like you to add
to. I'm using Obsidian and Markdown for them.

>  1 Ingestion & Indexing: Map all concepts from the notes.
>  2 Simulation & Verification: Test the rules from the notes against the structure of the
>    code.
>  3 Output: Present a structured report detailing the theoretical consensus, identified
>    inconsistencies, and proposed next steps for formalization.

Okay, I'm building a state system for you so that we can restart sessions with new tools
added. TODOs for me: building a tool for reading from the notes and from the tako repo, do
you need experimental tooling as well? I'm imagining being able to run experiments with the
tako interpreter might be helpful but maybe that's jumping the gun. Thoughts?
"""
