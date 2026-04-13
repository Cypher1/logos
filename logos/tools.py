from dataclasses import dataclass
from pathlib import Path
import os

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
class Memory:
    dir: Path

    def __post_init__(self):
        os.makedirs(self.dir, exist_ok=True)

    def list_memories(self) -> str:
        print(f"Listing memories {self.dir}...")
        from os import walk

        f = []
        for (_dirpath, _dirnames, filenames) in walk(self.dir):
            f.extend(filenames)
            break
        return "\n".join(f)

    def read_memory(self, name: str) -> str | None:
        print(f"Reading memory {self.dir / name}...")
        try:
            with safe_open(self.dir / f"{name}", "r", self.dir) as f:
                data = f.read()
            return data
        except FileNotFoundError:
            return None

    def write_memory(self, name: str, data: str) -> bool:
        print(f"Saving {self.dir / name}...")
        with safe_open(self.dir / f"{name}", "w", self.dir) as f:
            f.write(data)
        return True

    def read_memory_bytes(self, name: str) -> bytes | None:
        print(f"Reading binary memory {self.dir / name}...")
        try:
            with safe_open_binary(self.dir / f"{name}", "rb", self.dir) as f:
                data = f.read()
            return data
        except FileNotFoundError:
            return None

    def write_memory_bytes(self, name: str, data: bytes) -> bool:
        print(f"Saving {self.dir / name}...")
        with safe_open_binary(self.dir / f"{name}", "wb", self.dir) as f:
            f.write(data)
        return True
