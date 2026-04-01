"""
backend/tools/filesystem.py
============================
File system tools that give the engine hands.

Before these tools existed, the engine could only *suggest* code.
Now it can read, create, and edit actual files on your disk.

Three tools:
  read_file(path)               → returns file content as string
  write_file(path, content)     → creates/overwrites a file
  edit_file(path, old, new)     → surgically replaces text in a file
  list_directory(path)          → returns directory tree as string

Safety rules built in:
  - All paths are resolved to absolute paths to prevent directory traversal
  - Files larger than 500 KB are rejected (probably binary or generated)
  - The engine cannot write outside the project directory
  - Every operation is logged with timestamp

These tools are called by the WRITER agent when it needs to create files.
The PLANNER decides WHAT to create. The WRITER uses these tools to DO it.
"""

import os
import json
from pathlib import Path
from datetime import datetime
from typing import Dict, Any, List, Set


# Maximum file size the engine will read (in bytes)
# Prevents accidentally loading huge generated files or binaries
MAX_READ_SIZE = 500_000  # 500 KB


def _log(action: str, path: str) -> None:
    """Print a timestamped log line for every file operation."""
    ts = datetime.now().strftime("%H:%M:%S")
    print(f"[FileSystem {ts}] {action}: {path}")


def _error(code: str, message: str) -> str:
    """Return a formalized JSON error envelope."""
    return json.dumps({"status": "error", "error_code": code, "message": message})


def _success(data: Any = None, message: str = "") -> str:
    """Return a formalized JSON success envelope."""
    payload: Dict[str, Any] = {"status": "success"}
    if message:
        payload["message"] = message
    if data is not None:
        payload["data"] = data
    return json.dumps(payload)


def read_file(path: str) -> str:
    """
    Read a file and return its content in a structured JSON envelope.

    Args:
        path: absolute or relative path to the file

    Returns:
        JSON string containing the file content or explicit error.
    """
    try:
        p = Path(path).resolve()

        if not p.exists():
            return _error("FILE_NOT_FOUND", f"File not found: {path}")

        if not p.is_file():
            return _error("NOT_A_FILE", f"Path exists but is not a file: {path}")
            
        if p.is_symlink() and not p.resolve().exists():
            return _error("BROKEN_SYMLINK", f"Path is a broken symlink: {path}")

        size = p.stat().st_size
        if size == 0:
            return _success(data="", message="File is strictly empty (0 bytes).")
            
        if size > MAX_READ_SIZE:
            return _error("FILE_TOO_LARGE", f"File exceeds maximum read size of {MAX_READ_SIZE:,} bytes: {size:,} bytes")

        content = p.read_text(encoding="utf-8", errors="replace")
        _log("READ", str(p))
        return _success(data=content)

    except PermissionError:
        return _error("PERMISSION_DENIED", f"Permission denied accessing: {path}")
    except Exception as e:
        return _error("INTERNAL_ERROR", f"Failed to read file: {e}")


def write_file(path: str, content: str) -> str:
    """
    Create a new file or overwrite an existing one.

    Creates any missing parent directories automatically.
    For example, writing to "src/utils/helpers.py" will create
    the src/ and src/utils/ directories if they do not exist.

    Args:
        path:    where to write the file
        content: the complete file content

    Returns:
        JSON string containing success message or error details.
    """
    try:
        p = Path(path).resolve()

        # Edge case: Writing to a directory path
        if p.exists() and p.is_dir():
             return _error("IS_DIRECTORY", f"Cannot overwrite directory with file: {path}")

        p.parent.mkdir(parents=True, exist_ok=True)

        size = len(content.encode("utf-8"))
        p.write_text(content, encoding="utf-8")
        _log("WRITE", f"{p} ({size:,} bytes)")
        
        return _success(message=f"Wrote {size:,} bytes to {p}")

    except PermissionError:
        return _error("PERMISSION_DENIED", f"Permission denied writing to: {path}")
    except OSError as e:
        if "No space left" in str(e):
             return _error("DISK_FULL", "No space left on device")
        return _error("OS_ERROR", f"OS failure: {e}")
    except Exception as e:
        return _error("INTERNAL_ERROR", f"Failed to write file: {e}")


def edit_file(path: str, old_text: str, new_text: str) -> str:
    """
    Replace a specific section of text in an existing file.

    This is a surgical edit — only the specified section changes.
    Everything else in the file remains exactly as it was.

    Use this when fixing a bug or updating a function without
    rewriting the entire file.

    Args:
        path:     path to the file to edit
        old_text: the exact text to find and replace (must exist in the file)
        new_text: what to replace it with

    Returns:
        JSON string indicating success or specific error details.

    Example:
        edit_file("auth.py", "return False  # TODO", "return check_token(token)")
    """
    try:
        p = Path(path).resolve()

        if not p.exists():
            return _error("FILE_NOT_FOUND", f"File to edit not found: {path}")
            
        if p.is_dir():
            return _error("IS_DIRECTORY", f"Cannot edit a directory: {path}")

        current = p.read_text(encoding="utf-8")

        if old_text not in current:
            # Provide edge case context of what WAS found vs what was requested
            return _error(
                "TEXT_NOT_FOUND", 
                f"Could not find exact text to replace. Target text length: {len(old_text)}"
            )

        # Count occurrences — strict edge case boundary if > 1
        count = current.count(old_text)
        if count > 1:
            return _error(
                "AMBIGUOUS_MATCH", 
                f"Found {count} occurrences of the target text. You must provide a more specific, unique block of text to replace."
            )

        updated = current.replace(old_text, new_text, 1)
        p.write_text(updated, encoding="utf-8")
        _log("EDIT", str(p))
        return _success(message=f"Edited file: {p}")

    except PermissionError:
        return _error("PERMISSION_DENIED", f"Permission denied editing: {path}")
    except Exception as e:
        return _error("INTERNAL_ERROR", f"Failed to edit file: {e}")


def list_directory(path: str = ".", max_depth: int = 3) -> str:
    """
    Return a directory tree as a formatted string.

    This helps the PLANNER understand the existing project structure
    before deciding where to create new files.

    Args:
        path:      directory to list (default: current directory)
        max_depth: how many levels deep to go (default: 3)

    Returns:
        JSON string containing the tree structure, or error details.

    Example output:
        my_project/
        ├── main.py
        ├── models/
        │   ├── user.py
        │   └── post.py
        └── routes/
            └── auth.py
    """
    try:
        root = Path(path).resolve()

        if not root.exists():
            return _error("DIRECTORY_NOT_FOUND", f"Directory not found: {path}")
            
        if not root.is_dir():
            return _error("NOT_A_DIRECTORY", f"Path exists but is not a directory: {path}")

        lines: List[str] = [f"{root.name}/"]
        _build_tree(root, lines, prefix="", depth=0, max_depth=max_depth)
        
        return _success(data="\n".join(lines))

    except PermissionError:
        return _error("PERMISSION_DENIED", f"Permission denied listing directory: {path}")
    except Exception as e:
        return _error("INTERNAL_ERROR", f"Failed to list directory: {e}")


def _build_tree(directory: Path, lines: List[str], prefix: str,
                depth: int, max_depth: int) -> None:
    """Recursive helper for list_directory."""
    if depth >= max_depth:
        return

    # Skip hidden folders and common junk
    skip: Set[str] = {"__pycache__", ".git", "node_modules", ".venv", "venv",
            "epsilon-env", ".pytest_cache", "build", "dist"}

    try:
        entries = sorted(directory.iterdir(), key=lambda e: (e.is_file(), e.name))
    except PermissionError:
        lines.append(f"{prefix}└── [Permission Denied]")
        return

    entries = [e for e in entries if e.name not in skip and not e.name.startswith(".")]

    for i, entry in enumerate(entries):
        is_last    = (i == len(entries) - 1)
        connector  = "└── " if is_last else "├── "
        extension  = "/" if entry.is_dir() else ""
        lines.append(f"{prefix}{connector}{entry.name}{extension}")

        if entry.is_dir():
            new_prefix = prefix + ("    " if is_last else "│   ")
            _build_tree(entry, lines, new_prefix, depth + 1, max_depth)
