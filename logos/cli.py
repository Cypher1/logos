import sys
from pathlib import Path
from prompt_toolkit import PromptSession

from ollama import Message
from rich.console import Console

from logos.bot import Bot

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
