from dataclasses import dataclass
from pathlib import Path
import os

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
        os.mkdir(self.dir)

    def list_memories(self) -> str:
        print(f"Listing memories {self.dir}...")
        from os import walk

        f = []
        for (_dirpath, _dirnames, filenames) in walk(self.dir):
            f.extend(filenames)
            break
        return "\n".join(f)

    def read_memory(self, name: str) -> str:
        print(f"Reading memory {self.dir / self.name}...")
        try:
            with open(self.dir / f"{name}.json", "r") as f:
                data = f.read()
            return data
        except FileNotFoundError:
            return f"No memory file {name}"

    def add_to_memory(self, name: str, data: str) -> str:
        print(f"Saving {self.dir / name}...")
        with open(self.dir / f"{name}.json", "w") as f:
            f.write(data)
        return "Done"
