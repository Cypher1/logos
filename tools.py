from collections.abc import Callable
from dataclasses import dataclass, field
from typing import Literal

from ollama import ChatResponse, Message, chat
from rich.console import Console
from rich.markdown import Markdown
from rich.syntax import Syntax

MODEL='gemma4:latest'


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
class State:
    model: str = MODEL
    messages: list[Message] = field(default_factory=list)
    tools: dict[str, Callable] = field(default_factory=dict)
    think: bool | Literal['low', 'medium', 'high'] = True

    def add_tool(self, func) -> 'State':
        self.tools[func.__name__] = func
        return self

    def add_message(self, role: str, content: str, tool_name: str | None = None) -> 'State':
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


def run():
    console = Console()

    assistant = State() \
        .add_tool(get_temperature) \
        .add_tool(get_conditions)
    assistant.add_message(
        'user',
        str('What are the current weather conditions and temperature in New York and London?')
    )

    assistant.get_response()
    assistant.get_response()

    #with open("README.md") as readme:
    for message in assistant.messages:
        if message.tool_name and message.content:
            out = Syntax(message.content, "python", theme="monokai")
            console.print(out, style='red')
        else:
            if message.thinking:
                console.print(message.role, style='red', end='')
                console.print(' ', end='')
                console.print(message.thinking, style='yellow')
            if message.content:
                console.print(message.role, style='red', end='')
                console.print(' ', end='')
                out = Markdown(message.content)
                console.print(out, style='green')
        if message.images:
            console.print(message.images, style='red')

if __name__ == '__main__':
    run()
