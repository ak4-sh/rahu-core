from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


DEFAULT_ROOT = Path(".")


@dataclass(slots=True)
class Stat:
    path: Path
    lines: int
    words: int
    bytes: int

    @property
    def density(self) -> float:
        if self.lines == 0:
            return 0.0
        return self.words / self.lines


def is_python_file(path: Path) -> bool:
    return path.suffix in {".py", ".pyi"}


def iter_python_files(root: Path) -> Iterable[Path]:
    for path in root.rglob("*"):
        if path.is_file() and is_python_file(path):
            yield path


def analyze_file(path: Path) -> Stat:
    text = path.read_text(encoding="utf-8")
    return Stat(
        path=path,
        lines=text.count("\n") + (0 if text.endswith("\n") else 1),
        words=len(text.split()),
        bytes=len(text.encode("utf-8")),
    )


def summarize(stats: Iterable[Stat]) -> dict[str, float]:
    items = list(stats)
    total_lines = sum(item.lines for item in items)
    total_words = sum(item.words for item in items)
    total_bytes = sum(item.bytes for item in items)
    avg_density = sum(item.density for item in items) / len(items) if items else 0.0
    return {
        "files": float(len(items)),
        "lines": float(total_lines),
        "words": float(total_words),
        "bytes": float(total_bytes),
        "avg_density": avg_density,
    }


def render_report(summary: dict[str, float]) -> str:
    lines = [
        f"files={summary['files']:.0f}",
        f"lines={summary['lines']:.0f}",
        f"words={summary['words']:.0f}",
        f"bytes={summary['bytes']:.0f}",
        f"avg_density={summary['avg_density']:.2f}",
    ]
    return "\n".join(lines)


def collect_report(root: Path = DEFAULT_ROOT) -> str:
    stats = [analyze_file(path) for path in iter_python_files(root)]
    report = render_report(summarize(stats))
    note = f"scanned={root!s} count={len(stats)}"
    return f"{report}\n{note}"


def main() -> None:
    root = DEFAULT_ROOT
    report = collect_report(root)
    print(report)


if __name__ == "__main__":
    main()
