import asyncio
import inspect
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

import aiohttp
from ollama import AsyncClient, Message
from prompt_toolkit import PromptSession
from rich.console import Console

import logos.tools as tools
from logos.bot import Bot

COMMAND_LEADER = "/"


@dataclass
class Cli:
    console: Console
    prompt: PromptSession[str]
    sender: tools.Sender

    async def handle_command(self, assistant: Bot, command: str, args: list[str]) -> bool:
        if command == "quit":
            return True
        if command == "model":
            self.console.print(assistant.model, style="yellow")
        elif command == "tools":
            for k, v in assistant.tool_set.items():
                self.console.print(k, style="red", end="")
                self.console.print(
                    f"{inspect.signature(v)}",
                    style="yellow",
                    end="",
                )
                doc = f"  # {v.__doc__}" if v.__doc__ is not None else ""
                self.console.print(doc, style="green")
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
                        self.console.print(e)
                else:
                    value = assistant.get(key)
                self.console.print(f"{key} = {value}", style="yellow")
            else:
                for key, value in asdict(assistant).items():
                    # Allow the assistant code to skip some keys that shouldn't be user settable
                    if key in assistant.skip_fields():
                        continue
                    self.console.print(f"{key} = {value}", style="yellow")
        return False

    async def run_step(self, assistant: Bot):
        self.console.clear()
        for message in assistant.messages[-assistant.window_size :]:
            assistant.render_message(self.console, message)

        last = assistant.messages[-1] if assistant.messages else None
        if last is None or (last.role != "user" and last.role != "tool" and last.content):
            assistant.user_interrupt = True

        if assistant.user_interrupt:
            assistant.user_interrupt = False
            self.console.print("user", style="red")
            response = COMMAND_LEADER
            # TODO: Consider using a proper arg parser for this, allowing different data types etc.
            while response.startswith(COMMAND_LEADER):
                # Currently only supports keys that do not include spaces and integer setting values.
                args = response[len(COMMAND_LEADER) :].split(" ")
                command = args.pop(0)
                quit = await self.handle_command(assistant, command, args)
                if quit:
                    break
                response = await self.prompt.prompt_async("> ")
            if response:
                await assistant.add_message(Message(role="user", content=response))
        else:
            await assistant.get_response()

    async def run(self):
        # Load in the previous session
        logos_dir = Path.home() / ".logos"
        logos_dir.mkdir(parents=True, exist_ok=True)

        session_dir = logos_dir / "latest"

        # TODO: Consider trying the cloud API in some cases.
        client = AsyncClient()

        assistant = Bot(state_file=session_dir / "chat.jsonl", sender=self.sender, client=client)

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
        assistant.add_tool(tools.Sender.send_nfty_notification, instance=self.sender)

        assistant.load_state()

        while True:
            try:
                await self.run_step(assistant)
            except KeyboardInterrupt:
                assistant.user_interrupt = True
                # TODO: System messages through messages log without saving.
                self.console.print("Allowing interrupt (use Control-D for quit)", style="yellow")
            except EOFError:
                break
            except Exception as e:
                # TODO: Make code modifiable and reloadable at runtime?
                # TODO: System messages through messages log without saving.
                print(e, file=sys.stderr)
                raise e


async def amain():
    console = Console()
    prompt = PromptSession()

    async with aiohttp.ClientSession() as http_session:
        sender = tools.Sender(http_session)

        cli = Cli(
            console,
            prompt,
            sender,
        )

        await cli.run()


def main():
    asyncio.run(amain())
