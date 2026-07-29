"""Data processing pipeline with typing, generators, and comprehensions."""

from dataclasses import dataclass
from typing import Iterator, Optional


@dataclass
class Record:
    id: int
    name: str
    value: float
    tags: list[str]


class DataSource:
    def __init__(self, records: list[Record]) -> None:
        self.records = records

    def filter_by_tag(self, tag: str) -> Iterator[Record]:
        return (r for r in self.records if tag in r.tags)

    def aggregate(self, key: str) -> dict[str, float]:
        result: dict[str, float] = {}
        for r in self.records:
            k = getattr(r, key, "unknown")
            if k not in result:
                result[k] = 0.0
            result[k] += r.value
        return result


def parse_csv(text: str) -> list[Record]:
    records: list[Record] = []
    for line in text.strip().splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        parts = [p.strip() for p in line.split(",")]
        if len(parts) < 3:
            continue
        record = Record(
            id=int(parts[0]),
            name=parts[1],
            value=float(parts[2]),
            tags=parts[3:] if len(parts) > 3 else [],
        )
        records.append(record)
    return records


def generate_report(source: DataSource, min_value: float = 0.0) -> None:
    summary = source.aggregate("name")
    for name, total in sorted(summary.items(), key=lambda x: -x[1]):
        if total >= min_value:
            print(f"{name}: {total:.2f}")