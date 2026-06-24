#!/usr/bin/env python3
"""Explore OpenAlex snapshot partition behavior.

The main question this answers is whether a work ID appears in multiple
`updated_date=YYYY-MM-DD` partitions. If it does, a newest-first import with
insert-only semantics could keep the newest record and ignore older copies.
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import re
import sqlite3
from collections.abc import Iterator
from dataclasses import dataclass
from pathlib import Path


DEFAULT_ROOT = Path("/mnt/ssd_data/openalex/openalex-snapshot")
ID_RE = re.compile(rb'"id"\s*:\s*"([^"]+)"')
UPDATED_DATE_RE = re.compile(rb'"updated_date"\s*:\s*"([^"]+)"')


@dataclass(frozen=True)
class Partition:
    date: str
    path: Path


@dataclass
class Duplicate:
    work_id: str
    first_partition: str
    duplicate_partition: str
    first_updated_date: str | None
    duplicate_updated_date: str | None
    same_payload: bool


def find_partitions(root: Path, entity: str) -> list[Partition]:
    entity_dir = root / "data" / entity
    partitions = []
    for path in entity_dir.glob("updated_date=*"):
        if path.is_dir():
            partitions.append(Partition(path.name.removeprefix("updated_date="), path))
    return sorted(partitions, key=lambda p: p.date)


def iter_files(
    partitions: list[Partition],
    order: str,
    max_partitions: int | None,
    max_files_per_partition: int | None,
) -> Iterator[tuple[Partition, Path]]:
    if order == "newest":
        partitions = list(reversed(partitions))

    if max_partitions is not None:
        partitions = partitions[:max_partitions]

    for partition in partitions:
        files = sorted(partition.path.glob("part_*.gz"))
        if max_files_per_partition is not None:
            files = files[:max_files_per_partition]
        for file_path in files:
            yield partition, file_path


def payload_hash(raw_line: bytes) -> str:
    return hashlib.blake2b(raw_line, digest_size=16).hexdigest()


def iter_records(
    partition: Partition,
    file_path: Path,
    sample_lines_per_file: int | None,
    parse_json: bool,
) -> Iterator[tuple[str, str | None, str, bytes]]:
    with gzip.open(file_path, "rb") as handle:
        for i, raw_line in enumerate(handle, start=1):
            if sample_lines_per_file is not None and i > sample_lines_per_file:
                break

            if parse_json:
                record = json.loads(raw_line)
                work_id = record.get("id")
                updated_date = record.get("updated_date")
            else:
                id_match = ID_RE.search(raw_line)
                if not id_match:
                    continue
                work_id = id_match.group(1).decode("utf-8")
                updated_date_match = UPDATED_DATE_RE.search(raw_line)
                updated_date = (
                    updated_date_match.group(1).decode("utf-8")
                    if updated_date_match
                    else None
                )

            if work_id:
                yield work_id, updated_date, partition.date, raw_line


def print_summary(root: Path, entity: str) -> None:
    partitions = find_partitions(root, entity)
    total_files = 0
    total_bytes = 0

    print(f"root: {root}")
    print(f"entity: {entity}")
    print(f"partitions: {len(partitions)}")
    if partitions:
        print(f"oldest partition: {partitions[0].date}")
        print(f"newest partition: {partitions[-1].date}")

    for partition in partitions:
        files = list(partition.path.glob("part_*.gz"))
        total_files += len(files)
        total_bytes += sum(path.stat().st_size for path in files)

    print(f"gzip files: {total_files}")
    print(f"compressed bytes: {total_bytes:,}")
    print(f"compressed GiB: {total_bytes / 1024**3:.2f}")


def scan_duplicates_in_memory(args: argparse.Namespace) -> tuple[int, int, list[Duplicate]]:
    partitions = find_partitions(args.root, args.entity)
    seen: dict[str, tuple[str, str | None, str]] = {}
    duplicates: list[Duplicate] = []
    scanned = 0

    for partition, file_path in iter_files(
        partitions,
        args.order,
        args.max_partitions,
        args.max_files_per_partition,
    ):
        for work_id, updated_date, partition_date, raw_line in iter_records(
            partition,
            file_path,
            args.sample_lines_per_file,
            args.parse_json,
        ):
            scanned += 1
            current_hash = payload_hash(raw_line)
            previous = seen.get(work_id)
            if previous is None:
                seen[work_id] = (partition_date, updated_date, current_hash)
            else:
                first_partition, first_updated_date, first_hash = previous
                duplicates.append(
                    Duplicate(
                        work_id=work_id,
                        first_partition=first_partition,
                        duplicate_partition=partition_date,
                        first_updated_date=first_updated_date,
                        duplicate_updated_date=updated_date,
                        same_payload=first_hash == current_hash,
                    )
                )
                if len(duplicates) >= args.max_duplicates:
                    return scanned, len(seen), duplicates

            if args.max_records is not None and scanned >= args.max_records:
                return scanned, len(seen), duplicates

    return scanned, len(seen), duplicates


def scan_duplicates_sqlite(args: argparse.Namespace) -> tuple[int, int, list[Duplicate]]:
    partitions = find_partitions(args.root, args.entity)
    db_path = args.state_db
    if db_path.exists() and args.reset_state_db:
        db_path.unlink()

    connection = sqlite3.connect(db_path)
    connection.execute("PRAGMA journal_mode = WAL")
    connection.execute("PRAGMA synchronous = NORMAL")
    connection.execute(
        """
        CREATE TABLE IF NOT EXISTS seen (
            work_id TEXT PRIMARY KEY,
            partition_date TEXT NOT NULL,
            updated_date TEXT,
            payload_hash TEXT NOT NULL
        )
        """
    )

    scanned = 0
    duplicates: list[Duplicate] = []

    for partition, file_path in iter_files(
        partitions,
        args.order,
        args.max_partitions,
        args.max_files_per_partition,
    ):
        with connection:
            for work_id, updated_date, partition_date, raw_line in iter_records(
                partition,
                file_path,
                args.sample_lines_per_file,
                args.parse_json,
            ):
                scanned += 1
                current_hash = payload_hash(raw_line)
                row = connection.execute(
                    "SELECT partition_date, updated_date, payload_hash FROM seen WHERE work_id = ?",
                    (work_id,),
                ).fetchone()
                if row is None:
                    connection.execute(
                        """
                        INSERT INTO seen (work_id, partition_date, updated_date, payload_hash)
                        VALUES (?, ?, ?, ?)
                        """,
                        (work_id, partition_date, updated_date, current_hash),
                    )
                else:
                    first_partition, first_updated_date, first_hash = row
                    duplicates.append(
                        Duplicate(
                            work_id=work_id,
                            first_partition=first_partition,
                            duplicate_partition=partition_date,
                            first_updated_date=first_updated_date,
                            duplicate_updated_date=updated_date,
                            same_payload=first_hash == current_hash,
                        )
                    )
                    if len(duplicates) >= args.max_duplicates:
                        unique = connection.execute("SELECT COUNT(*) FROM seen").fetchone()[0]
                        return scanned, unique, duplicates

                if args.max_records is not None and scanned >= args.max_records:
                    unique = connection.execute("SELECT COUNT(*) FROM seen").fetchone()[0]
                    return scanned, unique, duplicates

    unique = connection.execute("SELECT COUNT(*) FROM seen").fetchone()[0]
    return scanned, unique, duplicates


def print_duplicate_report(scanned: int, unique: int, duplicates: list[Duplicate]) -> None:
    print(f"records scanned: {scanned:,}")
    print(f"unique work IDs seen: {unique:,}")
    print(f"duplicate work IDs found: {len(duplicates):,}")

    for duplicate in duplicates[:20]:
        print(
            "duplicate:",
            duplicate.work_id,
            f"first_partition={duplicate.first_partition}",
            f"duplicate_partition={duplicate.duplicate_partition}",
            f"first_updated_date={duplicate.first_updated_date}",
            f"duplicate_updated_date={duplicate.duplicate_updated_date}",
            f"same_payload={duplicate.same_payload}",
        )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    parser.add_argument("--entity", default="works")

    subparsers = parser.add_subparsers(dest="command", required=True)

    subparsers.add_parser("summary", help="summarize snapshot partitions")

    duplicates = subparsers.add_parser(
        "duplicates",
        help="scan for IDs repeated across updated_date partitions",
    )
    duplicates.add_argument("--order", choices=["oldest", "newest"], default="newest")
    duplicates.add_argument("--max-partitions", type=int)
    duplicates.add_argument("--max-files-per-partition", type=int)
    duplicates.add_argument("--max-records", type=int)
    duplicates.add_argument("--sample-lines-per-file", type=int)
    duplicates.add_argument("--max-duplicates", type=int, default=20)
    duplicates.add_argument(
        "--parse-json",
        action="store_true",
        help="parse full JSON records instead of using the faster byte scanner",
    )
    duplicates.add_argument(
        "--sqlite",
        action="store_true",
        help="store seen IDs in SQLite instead of memory",
    )
    duplicates.add_argument(
        "--state-db",
        type=Path,
        default=Path("/tmp/openalex_dump_explore.sqlite"),
    )
    duplicates.add_argument("--reset-state-db", action="store_true")

    return parser.parse_args()


def main() -> None:
    args = parse_args()
    if args.command == "summary":
        print_summary(args.root, args.entity)
        return

    if args.command == "duplicates":
        if args.sqlite:
            scanned, unique, duplicates = scan_duplicates_sqlite(args)
        else:
            scanned, unique, duplicates = scan_duplicates_in_memory(args)
        print_duplicate_report(scanned, unique, duplicates)
        return

    raise AssertionError(f"unhandled command: {args.command}")


if __name__ == "__main__":
    main()
