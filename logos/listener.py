import asyncio
import json
from collections.abc import Awaitable, Callable
from dataclasses import dataclass, field

import aiohttp
from ollama import Message


@dataclass
class NtfyListener:
    topic: str
    open = False
    observers: list[Callable[[Message], Awaitable[None]]] = field(default_factory=list)

    @property
    def json_url(self) -> str:
        return f"https://ntfy.sh/{self.topic}/json"

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
                    await asyncio.gather(*(obs(message) for obs in self.observers))

                # Raw message
                # print(f"Unknown message (open = {self.open})", obj)


async def amain():
    listener = NtfyListener("ellie_logos")

    async def printer(message: Message):
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
