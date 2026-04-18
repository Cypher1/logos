import asyncio
import json
from collections.abc import Callable
from dataclasses import dataclass, field
from multiprocessing import Process
from typing import Protocol, TypeVar

import aiohttp
from ollama import Message

T = TypeVar("T")
U = TypeVar("U")


class PipeConnectionI[T, U](Protocol):
    def send(self, obj: T): ...
    def recv(self) -> U: ...
    def poll(self, timeout: float | None = 0) -> bool: ...
    def close(self): ...


@dataclass
class NtfyListener:
    topic: str
    open = False
    observers: list[Callable[[Message], None]] = field(default_factory=list)
    child_conn: PipeConnectionI[Message, None] | None = field(default=None)
    process: Process | None = field(default=None, init=False)

    @property
    def json_url(self) -> str:
        return f"https://ntfy.sh/{self.topic}/json"

    def run_as_process(self) -> "NtfyListener":
        def listen():
            assert self.child_conn is not None
            self.observers.append(self.child_conn.send)
            asyncio.run(self.start())

        assert self.process is None
        self.process = Process(target=listen)
        self.process.start()
        return self

    def join(self):
        if self.child_conn:
            self.child_conn.close()
        if self.process:
            self.process.join()

    async def start(self):
        async with aiohttp.ClientSession() as http_session:
            await self.listen(http_session)

    async def listen(self, client: aiohttp.ClientSession):
        async with client.get(self.json_url, timeout=None) as resp:
            async for line in resp.content:
                obj = json.loads(line)
                content: str = str(obj.get("message"))
                event = obj["event"]

                if event == "open":  # success
                    self.open = True
                elif event == "keepalive":
                    pass
                elif event == "message":
                    if ": " in content:
                        role, content = content.split(": ", 1)
                    else:
                        role = "user"

                    message = Message(
                        role=role,
                        content=content,
                    )
                    for obs in self.observers:
                        obs(message)

                # Raw message
                # print(f"Unknown message (open = {self.open})", obj)


async def amain():
    listener = NtfyListener("ellie_logos")

    def printer(message: Message):
        print(message.role)
        print(message.content)

    listener.observers.append(printer)

    async with aiohttp.ClientSession() as session:
        await listener.listen(session)
        await session.close()


def main():
    asyncio.run(amain())


if __name__ == "__main__":
    main()
