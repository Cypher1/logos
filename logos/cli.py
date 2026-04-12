import sys
from pathlib import Path
import os

from prompt_toolkit import PromptSession
from rich.console import Console

import logos.tools as tools
from logos.bot import Bot

MODEL = "gemma4:latest"


def main():
    console = Console()
    session = PromptSession()

    # Load in the previous session
    logos_dir = Path.home() / ".logos"

    os.mkdir(logos_dir)

    memory_file = logos_dir / "memory"
    state_file = logos_dir / "chat.json"

    memory = tools.Memory(memory_file)
    assistant = Bot(MODEL)
    assistant.add_tool(tools.Memory.read_memory, instance=memory)
    assistant.add_tool(tools.Memory.add_to_memory, instance=memory)
    assistant.add_tool(tools.get_temperature)
    assistant.add_tool(tools.get_conditions)

    assistant.load_state(state_file)

    while True:
        try:
            console.clear()
            for message in assistant.messages:
                assistant.render_message(console, message)

            last = assistant.messages[-1] if assistant.messages else None
            if last is None or (
                last.role != "user" and last.role != "tool" and last.content
            ):
                console.print("user", style="red")
                response = session.prompt("> ")
                if response:
                    assistant.add_message("user", response)
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
