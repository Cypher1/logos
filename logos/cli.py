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

    main_memory = tools.Memory(session_dir / "memory")
    assistant.add_tool(tools.Memory.read_memory, instance=main_memory, namespace="main_memory")
    assistant.add_tool(tools.Memory.write_memory, instance=main_memory, namespace="main_memory")
    assistant.add_tool(tools.Memory.list_memories, instance=main_memory, namespace="main_memory")

    repo_memory = tools.Memory(session_dir / "tako")
    assistant.add_tool(tools.Memory.read_memory, instance=repo_memory, namespace="repo_memory")
    assistant.add_tool(tools.Memory.write_memory, instance=repo_memory, namespace="repo_memory")
    assistant.add_tool(tools.Memory.list_memories, instance=repo_memory, namespace="repo_memory")

    assistant.add_tool(tools.get_temperature)
    assistant.add_tool(tools.get_conditions)

    assistant.load_state()

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
                response = "/"
                while response.startswith("/"):
                    if response == "/model":
                        console.print(assistant.model, style="yellow")
                    if response == "/tools":
                        console.print("\n".join(assistant.tools.keys()), style="yellow")
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
            break
        except EOFError:
            break
        except Exception as e:
            print(e, file=sys.stderr)
            # Then quit

    # No longer needed as messages are added one by one.
    # assistant.save_state()
