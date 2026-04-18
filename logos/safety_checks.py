from io import TextIOWrapper
from pathlib import Path
from typing import TYPE_CHECKING, BinaryIO

if TYPE_CHECKING:
    from _typeshed import OpenBinaryMode, OpenTextMode


class ForbiddenPathError(Exception):
    def __init__(self, message, requested_path: Path):
        super().__init__(message)
        self.path = requested_path


def validate_requested_file_path(*, requested_path: Path, contained_directory: Path):
    # TODO: Test
    # WARNING: os.path.abspath does not resolve symlinks
    # This is intentional as a basic security measure to restrict read/write access.
    #
    # For more information see:
    # https://salvatoresecurity.com/preventing-directory-traversal-vulnerabilities-in-python/
    if contained_directory not in requested_path.resolve().parents:
        raise ForbiddenPathError(
            f"The requested path '{requested_path}' is forbidden",
            requested_path=requested_path,
        )


def safe_read(path: Path, contained_directory: Path) -> TextIOWrapper:
    return safe_open(path, mode="r", contained_directory=contained_directory)


def safe_append(path: Path, contained_directory: Path) -> TextIOWrapper:
    return safe_open(path, mode="a", contained_directory=contained_directory)


def safe_open(
    path: Path,
    mode: "OpenTextMode",
    contained_directory: Path,
) -> TextIOWrapper:
    # TODO: Test
    validate_requested_file_path(
        requested_path=path,
        contained_directory=contained_directory,
    )
    path.parent.mkdir(parents=True, exist_ok=True)
    return path.open(mode)


def safe_open_binary(
    path: Path,
    mode: "OpenBinaryMode",
    contained_directory: Path,
) -> BinaryIO:
    # TODO: Test
    validate_requested_file_path(
        requested_path=path,
        contained_directory=contained_directory,
    )
    path.parent.mkdir(parents=True, exist_ok=True)
    return path.open(mode)
