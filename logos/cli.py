import inspect
import os
import sys
from dataclasses import asdict
from pathlib import Path

from ollama import Message
from prompt_toolkit import PromptSession
from rich.console import Console

import logos.tools as tools
from logos.bot import Bot

COMMAND_LEADER = "/"


def main():
    console = Console()
    session = PromptSession()

    # Load in the previous session
    logos_dir = Path.home() / ".logos"
    os.makedirs(logos_dir, exist_ok=True)

    session_dir = logos_dir / "latest"

    assistant = Bot(state_file=session_dir / "chat.jsonl")

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
            for message in assistant.messages[-assistant.window_size :]:
                assistant.render_message(console, message)

            last = assistant.messages[-1] if assistant.messages else None
            if last is None or (
                last.role != "user" and last.role != "tool" and last.content
            ):
                user_interrupt = True

            if user_interrupt:
                user_interrupt = False
                console.print("user", style="red")
                response = COMMAND_LEADER
                # TODO: Consider using a proper arg parser for this, allowing different data types etc.
                while response.startswith(COMMAND_LEADER):
                    # Currently only supports keys that do not include spaces and integer setting values.
                    args = response[len(COMMAND_LEADER) :].split(" ")
                    command = args.pop(0)
                    if command == "quit":
                        break
                    elif command == "model":
                        console.print(assistant.model, style="yellow")
                    elif command == "tools":
                        for k, v in assistant.tools.items():
                            console.print(k, style="red", end="")
                            console.print(
                                f"{inspect.signature(v)}", style="yellow", end=""
                            )
                            doc = f"  # {v.__doc__}" if v.__doc__ is not None else ""
                            console.print(doc, style="green")
                    elif command == "save":
                        assistant.save_state()
                    elif command == "load":
                        assistant.load_state()
                    elif command == "set":
                        if args:
                            key = args[0]
                            if len(args) > 1:
                                value = args[1]
                                try:
                                    assistant.set(key, value)
                                except Exception as e:
                                    console.print(e)
                            else:
                                value = assistant.get(key)
                            console.print(f"{key} = {value}", style="yellow")
                        else:
                            for key, value in asdict(assistant).items():
                                # Allow the assistant code to skip some keys that shouldn't be user settable
                                if key in assistant.skip_fields():
                                    continue
                                console.print(f"{key} = {value}", style="yellow")
                    response = session.prompt("> ")
                if response:
                    assistant.add_message(Message(role="user", content=response))
            else:
                assistant.get_response()
        except KeyboardInterrupt:
            user_interrupt = True
            # TODO: System messages through messages log without saving.
            print("Allowing interrupt (use Control-D for quit)")
        except EOFError:
            break
        except Exception as e:
            # TODO: Make code modifiable and reloadable at runtime?
            # TODO: System messages through messages log without saving.
            print(e, file=sys.stderr)
            raise e
