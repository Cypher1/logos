import inspect
import sys
from pathlib import Path
import os

from ollama import Message
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
    os.makedirs(logos_dir, exist_ok=True)

    session_dir = logos_dir / "latest"

    assistant = Bot(MODEL, state_file = session_dir / "chat.jsonl")

    _main = tools.Memory(session_dir / "memory")
    assistant.add_tool(tools.Memory.read_memory, instance=_main, namespace="main")
    assistant.add_tool(tools.Memory.write_memory, instance=_main, namespace="main")
    assistant.add_tool(tools.Memory.list_memories, instance=_main, namespace="main")

    repo = tools.Memory(session_dir / "tako")
    assistant.add_tool(tools.Memory.read_memory, instance=repo, namespace="repo")
    assistant.add_tool(tools.Memory.write_memory, instance=repo, namespace="repo")
    assistant.add_tool(tools.Memory.list_memories, instance=repo, namespace="repo")

    notes = tools.Memory(session_dir / "notes")
    assistant.add_tool(tools.Memory.read_memory, instance=notes, namespace="notes")
    assistant.add_tool(tools.Memory.write_memory, instance=notes, namespace="notes")
    assistant.add_tool(tools.Memory.list_memories, instance=notes, namespace="notes")

    assistant.add_tool(tools.get_temperature)
    assistant.add_tool(tools.get_conditions)

    assistant.load_state()

    user_interrupt = True

    while True:
        try:
            console.clear()
            for message in assistant.messages:
                assistant.render_message(console, message)

            last = assistant.messages[-1] if assistant.messages else None
            if last is None or (
                last.role != "user" and last.role != "tool" and last.content
            ):
                user_interrupt = True

            if user_interrupt:
                user_interrupt = False
                console.print("user", style="red")
                response = "/"
                while response.startswith("/"):
                    if response == "/quit":
                        break
                    if response == "/model":
                        console.print(assistant.model, style="yellow")
                    if response == "/tools":
                        for k, v in assistant.tools.items():
                            console.print(k, style="red", end="")
                            console.print(f"{inspect.signature(v)}", style="yellow", end="")
                            doc = f"  # {v.__doc__}" if v.__doc__ is not None else ""
                            console.print(doc, style="green")
                    elif response == "/save":
                        assistant.save_state()
                    elif response == "/load":
                        assistant.load_state()

                    response = session.prompt("> ")
                if response:
                    assistant.add_message(Message(role="user", content=response))
            else:
                assistant.get_response()
        except KeyboardInterrupt:
            user_interrupt = True
            print("Allowing interrupt (use Control-D for quit)")
        except EOFError:
            break
        except Exception as e:
            print(e, file=sys.stderr)
            raise e
            # break
            # Then quit

    # No longer needed as messages are added one by one.
    # assistant.save_state()
