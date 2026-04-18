from dataclasses import dataclass
from pathlib import Path

import aiohttp
from ollama import Message

from logos.safety_checks import safe_open, safe_open_binary


def get_temperature(city: str) -> str:
    """Get the current temperature for a city

    Args:
      city: The name of the city

    Returns:
      The current temperature for the city
    """
    temperatures = {"New York": "22°C", "London": "15°C", "Tokyo": "18°C"}
    return temperatures.get(city, "Unknown")


def get_conditions(city: str) -> str:
    """Get the current weather conditions for a city

    Args:
      city: The name of the city

    Returns:
      The current weather conditions for the city
    """
    conditions = {"New York": "Partly cloudy", "London": "Rainy", "Tokyo": "Sunny"}
    return conditions.get(city, "Unknown")


@dataclass
class Sender:
    session: aiohttp.ClientSession

    async def send_nfty_notification(self, data: str, title: str, priority: str, tags: str) -> None:
        async with self.session.post(
            "https://ntfy.sh/ellie_logos",
            data=f"assistant: {data}",  # Prefix identifies sender
            headers={"Title": f"assistant: {title}", "Priority": priority, "Tags": tags},
        ) as response:
            _data = await response.text()
            # print(data)

    async def send_nfty_message(self, message: Message) -> None:
        async with self.session.post(
            "https://ntfy.sh/ellie_logos",
            data=f"{message.role}: {message.content}",  # Prefix identifies sender
            headers={"Markdown": "yes"},
        ) as response:
            _data = await response.text()
            # print(data)

    async def send_nfty_thinking(self, message: Message) -> None:
        async with self.session.post(
            "https://ntfy.sh/ellie_logos_thinking",
            data=message.model_dump_json(),
        ) as response:
            _data = await response.text()
            # print(data)


@dataclass
class ReadDir:
    dir: Path

    def ls(self, path=".") -> str:
        subdir = self.dir
        if path:
            subdir = subdir / f"{path}"
        files = [str(f.relative_to(subdir)) for f in subdir.iterdir()]
        return "\n".join(files)

    def read(self, path: str) -> str | None:
        try:
            with safe_open(self.dir / f"{path}", "r", self.dir) as f:
                return f.read()
        except FileNotFoundError:
            return None

    def read_bytes(self, path: str) -> bytes | None:
        try:
            with safe_open_binary(self.dir / f"{path}", "rb", self.dir) as f:
                return f.read()
        except FileNotFoundError:
            return None


@dataclass
class ReadWriteDir(ReadDir):
    def write(self, path: str, data: str) -> bool:
        with safe_open(self.dir / f"{path}", "w", self.dir) as f:
            f.write(data)
        return True

    def write_bytes(self, path: str, data: bytes) -> bool:
        with safe_open_binary(self.dir / f"{path}", "wb", self.dir) as f:
            f.write(data)
        return True
