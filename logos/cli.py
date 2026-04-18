import asyncio
import inspect
import signal
from dataclasses import dataclass
from multiprocessing import Pipe
from pathlib import Path

import aiohttp
from ollama import AsyncClient, Message
from prompt_toolkit import PromptSession
from rich.console import Console

import logos.tools as tools
from logos.bot import Bot
from logos.listener import NtfyListener

COMMAND_LEADER = "/"


@dataclass
class Cli:
    console: Console
    prompt: PromptSession[str]
    sender: tools.Sender
    http_session: aiohttp.ClientSession

    async def handle_command(self, assistant: Bot, command: str, args: list[str]):
        if command == "quit":
            assistant.shutdown = True
        elif command == "model":
            self.console.print(assistant.model, style="yellow")
        elif command == "tools":
            for tool, impl in assistant.tool_set.items():
                self.console.print(tool, style="red", end="")
                self.console.print(
                    f"{inspect.signature(impl)}",
                    style="yellow",
                    end="",
                )
                doc = f"  # {impl.__doc__}" if impl.__doc__ is not None else ""
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
                for key, value in assistant.__dict__.items():  # TODO: Cleanup
                    # Allow the assistant code to skip some keys that shouldn't be user settable
                    if key in assistant.skip_fields():
                        continue
                    self.console.print(f"{key} = {value}", style="yellow")

    async def run_step(self, assistant: Bot):
        assistant.render_messages(self.console)

        if not assistant.user_interrupt:
            await assistant.get_response(self.console)
            return

        self.console.print("user", style="red")
        # TODO: Put this in a task that yeilds / stops when `user_interrupt = False`
        # TODO: Consider using a proper arg parser for this, allowing different data types etc.

        response: str = "/"
        while response.startswith(COMMAND_LEADER):
            response = await self.prompt.prompt_async("> ")
            # Currently only supports keys that do not include spaces and integer setting values.
            args = response[len(COMMAND_LEADER) :].split(" ")
            command = args.pop(0)
            await self.handle_command(assistant, command, args)

        if not response:
            assistant.user_interrupt = False
            return

        await assistant.add_message(Message(role="user", content=response))

    async def start(self):
        # Load in the previous session
        logos_dir = Path.home() / ".logos"
        logos_dir.mkdir(parents=True, exist_ok=True)

        session_dir = logos_dir / "latest"

        # TODO: Consider trying the cloud API in some cases.
        client = AsyncClient()

        assistant = Bot(state_file=session_dir / "chat.jsonl", sender=self.sender, client=client)

        memory = tools.ReadWriteDir(session_dir / "memory")
        assistant.add_tool(tools.ReadWriteDir.read, instance=memory, namespace="memory")
        assistant.add_tool(tools.ReadWriteDir.write, instance=memory, namespace="memory")
        assistant.add_tool(tools.ReadWriteDir.ls, instance=memory, namespace="memory")

        tako = tools.ReadWriteDir(session_dir / "tako")
        assistant.add_tool(tools.ReadWriteDir.read, instance=tako, namespace="tako")
        assistant.add_tool(tools.ReadWriteDir.ls, instance=tako, namespace="tako")

        notes = tools.ReadWriteDir(session_dir / "notes")
        assistant.add_tool(tools.ReadWriteDir.read, instance=notes, namespace="notes")
        assistant.add_tool(tools.ReadWriteDir.ls, instance=notes, namespace="notes")

        assistant.add_tool(tools.get_temperature)
        assistant.add_tool(tools.get_conditions)

        # So that Logos can message directly
        assistant.add_tool(tools.Sender.send_nfty_notification, instance=self.sender)

        assistant.load_state()

        loop = asyncio.get_event_loop()

        def set_interrupt():
            assistant.user_interrupt = True
            self.console.print("Allowing interrupt (use /quit for quit)", style="yellow")

        loop.add_signal_handler(signal.SIGINT, set_interrupt)
        loop.add_signal_handler(signal.SIGTERM, set_interrupt)
        await self.run(assistant)

    async def run(self, assistant: Bot):
        reciever_conn, listener_conn = Pipe()

        with NtfyListener("ellie_logos", child_conn=listener_conn):
            while not assistant.shutdown:
                if reciever_conn.poll():
                    msg = reciever_conn.recv()
                    await assistant.store_message(msg)
                try:
                    await self.run_step(assistant)
                except KeyboardInterrupt:
                    if assistant.user_interrupt:
                        assistant.shutdown = True
                    else:
                        assistant.user_interrupt = True
                        # TODO: System messages through messages log without saving.
                        self.console.print("Allowing interrupt (use /quit for quit)", style="yellow")
                except EOFError:
                    assistant.user_interrupt = True
                    # TODO: System messages through messages log without saving.
                    self.console.print("Allowing interrupt (use /quit for quit)", style="yellow")


async def amain():
    # TODO: Switch to Textual
    # https://realpython.com/python-textual/#creating-your-first-textual-app
    console = Console(soft_wrap=True, tab_size=4)
    prompt = PromptSession()

    async with aiohttp.ClientSession() as http_session:
        sender = tools.Sender(http_session)

        cli = Cli(
            console,
            prompt,
            sender,
            http_session,
        )

        await cli.start()


def main():
    asyncio.run(amain())
