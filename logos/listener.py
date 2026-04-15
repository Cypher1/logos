import asyncio
import json
from dataclasses import dataclass

import aiohttp


@dataclass
class NtfyListener:
    topic: str
    open = False

    @property
    def json_url(self) -> str:
        return f"https://ntfy.sh/{self.topic}/json"

    async def listen(self, client: aiohttp.ClientSession):
        async with client.get(self.json_url, timeout=None) as resp:
            async for line in resp.content:
                obj = json.loads(line)
                message: str = str(obj.get("message"))
                event = obj["event"]

                if event == "open":  # success
                    self.open = True
                    continue
                if event == "keepalive":
                    continue
                if event == "message":
                    if ": " in message:
                        role, message = message.split(": ", 1)
                    else:
                        role = "user"
                    print(role)
                    print(message)
                    continue

                # Raw message
                print(f"Unknown message (open = {self.open})", obj)


async def amain():
    listener = NtfyListener("ellie_logos")

    async with aiohttp.ClientSession() as session:
        await listener.listen(session)
        await session.close()


def main():
    asyncio.run(amain())


if __name__ == '__main__':
    main()
