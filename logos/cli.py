import inspect
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
    logos_dir.mkdir(parents=True)

    session_dir = logos_dir / "latest"

    assistant = Bot(state_file=session_dir / "chat.jsonl")

    _main = tools.ReadWriteDir(session_dir / "memory")
    assistant.add_tool(tools.ReadWriteDir.read, instance=_main, namespace="main")
    assistant.add_tool(tools.ReadWriteDir.write, instance=_main, namespace="main")
    assistant.add_tool(tools.ReadWriteDir.list_files, instance=_main, namespace="main")

    repo = tools.ReadWriteDir(session_dir / "tako")
    assistant.add_tool(tools.ReadDir.read, instance=repo, namespace="repo")
    assistant.add_tool(tools.ReadDir.list_files, instance=repo, namespace="repo")

    notes = tools.ReadWriteDir(session_dir / "notes")
    assistant.add_tool(tools.ReadDir.read, instance=notes, namespace="notes")
    assistant.add_tool(tools.ReadDir.list_files, instance=notes, namespace="notes")

    assistant.add_tool(tools.get_temperature)
    assistant.add_tool(tools.get_conditions)

    # So that Logos can message directly
    assistant.add_tool(tools.send_nfty_notification)

    assistant.load_state()

    user_interrupt = True

    while True:
        try:
            console.clear()
            for message in assistant.messages[-assistant.window_size :]:
                assistant.render_message(console, message)

            last = assistant.messages[-1] if assistant.messages else None
            if last is None or (last.role != "user" and last.role != "tool" and last.content):
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
                    if command == "model":
                        console.print(assistant.model, style="yellow")
                    elif command == "tools":
                        for k, v in assistant.tool_set.items():
                            console.print(k, style="red", end="")
                            console.print(
                                f"{inspect.signature(v)}",
                                style="yellow",
                                end="",
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
