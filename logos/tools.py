from dataclasses import dataclass
from pathlib import Path

import requests

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


def send_nfty_notification(data: str, title: str, priority: str, tags: str) -> None:
    # TODO: Async
    requests.post(
        "https://ntfy.sh/ellie_logos",
        data=data,
        headers={"Title": title, "Priority": priority, "Tags": tags},
    )


def send_nfty_message(data: str) -> None:
    # TODO: Async
    requests.post(
        "https://ntfy.sh/ellie_logos",
        data=data,
    )


def send_nfty_thinking(data: str) -> None:
    # TODO: Async
    requests.post(
        "https://ntfy.sh/ellie_logos_thinking",
        data=data,
    )


@dataclass
class ReadDir:
    dir: Path

    def list_files(self) -> str:
        print(f"Listing files {self.dir}...")
        from os import walk

        f = []
        for _dirpath, _dirnames, filenames in walk(self.dir):
            f.extend(filenames)
            break
        return "\n".join(f)

    def read(self, name: str) -> str | None:
        print(f"Reading {self.dir / name}...")
        try:
            with safe_open(self.dir / f"{name}", "r", self.dir) as f:
                return f.read()
        except FileNotFoundError:
            return None

    def read_bytes(self, name: str) -> bytes | None:
        print(f"Reading binary {self.dir / name}...")
        try:
            with safe_open_binary(self.dir / f"{name}", "rb", self.dir) as f:
                return f.read()
        except FileNotFoundError:
            return None


@dataclass
class ReadWriteDir(ReadDir):
    def __post_init__(self):
        self.dir.mkdir(parents=True, exist_ok=True)

    def write(self, name: str, data: str) -> bool:
        print(f"Writing {self.dir / name}...")
        with safe_open(self.dir / f"{name}", "w", self.dir) as f:
            f.write(data)
        return True

    def write_bytes(self, name: str, data: bytes) -> bool:
        print(f"Writing bytes {self.dir / name}...")
        with safe_open_binary(self.dir / f"{name}", "wb", self.dir) as f:
            f.write(data)
        return True
