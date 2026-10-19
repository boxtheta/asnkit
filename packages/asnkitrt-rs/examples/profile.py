# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: MIT

"""Tell where the runtime for Rust spends its time, encoding or decoding.

Run it from the root of the repository:

    python3 packages/asnkitrt-rs/examples/profile.py run der decode
    python3 packages/asnkitrt-rs/examples/profile.py run uper encode --callers malloc

`run` builds `examples/profile.rs` with the `profiling` profile (release,
with symbols), runs it under the Time Profiler of Instruments
(`xctrace`, on macOS) and prints the functions that take the most time:
by themselves (self) and with what they call (inclusive). The trace and
its export are kept in `target/profile/`.

`report` prints the same of a profile taken otherwise: an export of
`xctrace` as `run` makes it, or folded stacks, one per line as
`main;caller;callee COUNT`, which `perf script | stackcollapse-perf.pl`
and other profilers write:

    perf record -g target/profiling/examples/profile der decode 2000000
    perf script | stackcollapse-perf.pl > der.folded
    python3 packages/asnkitrt-rs/examples/profile.py report der.folded

`--callers NAME` tells which functions of the runtime call the
functions with NAME in their name, as `malloc`, and `--stacks NAME` the
frames above them. Names are demangled with `rustfilt` if it is
installed (`cargo install rustfilt`).

To read the numbers: malloc, free, realloc and memmove are what
allocating costs, and so is `mach_absolute_time` under the allocator
of macOS. The linker folds functions with the same code into one, so a
`Vec` of one type may be named after another of the same size, and
`<deduplicated_symbol>` is one of those.
"""

import argparse
import collections
import platform
import re
import shutil
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / "target" / "profile"
BINARY = ROOT / "target" / "profiling" / "examples" / "profile"
# How many calls a run makes, for about 10 seconds of samples.
CALLS = {"encode": 2_000_000, "decode": 1_500_000}


def xctrace_stacks(path):
    """The stacks of the samples of an export of `xctrace`, leaf first."""
    frames: dict[str, str] = {}
    backtraces: dict[str, list[str]] = {}
    stacks: list[list[str]] = []

    def name_of(frame):
        if ref := frame.get("ref"):
            return frames[ref]

        name = frame.get("name", "?")
        frames[frame.get("id", "")] = name

        return name

    for row in ET.parse(path).getroot().iter("row"):
        backtrace = row.find("tagged-backtrace")

        if backtrace is None:
            continue

        if ref := backtrace.get("ref"):
            stack = backtraces[ref]
        else:
            stack = [name_of(frame) for frame in backtrace.findall("frame")]
            backtraces[backtrace.get("id", "")] = stack

        stacks.append(stack)

    return stacks


def folded_stacks(path):
    """The stacks of folded lines, `root;...;leaf COUNT`, leaf first."""
    stacks = []

    for line in Path(path).read_text().splitlines():
        frames, _, count = line.rpartition(" ")

        if frames and count.isdigit():
            stacks.extend([frames.split(";")[::-1]] * int(count))

    return stacks


def demangled(stacks):
    names = sorted({name for stack in stacks for name in stack})

    if shutil.which("rustfilt"):
        output = subprocess.run(
            ["rustfilt"],
            input="\n".join(names),
            capture_output=True,
            text=True,
            check=True,
        ).stdout.split("\n")
    else:
        print(
            "rustfilt is not installed: the names are not demangled.", file=sys.stderr
        )
        output = names

    clean = {}

    for name, readable in zip(names, output):
        readable = re.sub(r"::h[0-9a-f]{16}$", "", readable)
        clean[name] = re.sub(r"\[[0-9a-f]{16}\]", "", readable)

    return [[clean[name] for name in stack] for stack in stacks]


def report(stacks, top, callers, frames_of):
    stacks = demangled(stacks)
    # Only the samples of the calls, not those of loading and exiting.
    inside = [
        stack for stack in stacks if any("profile::main" in name for name in stack)
    ]
    stacks = inside or stacks
    total = len(stacks)

    if total == 0:
        sys.exit("There are no samples.")

    own: collections.Counter[str] = collections.Counter()
    within: collections.Counter[str] = collections.Counter()

    for stack in stacks:
        own[stack[0]] += 1

        for name in set(stack):
            within[name] += 1

    def show(title, counts, count):
        print(f"--- {title} ---")

        for name, samples in counts.most_common(count):
            print(f"{100 * samples / total:5.1f}%  {name[:160]}")

    print(f"{total} samples")
    show("self", own, top)
    # Below main, as the frames above it are in every sample.
    show(
        "inclusive",
        collections.Counter(
            {name: samples for name, samples in within.items() if samples < total}
        ),
        top,
    )

    for part in callers:
        counts: collections.Counter[str] = collections.Counter()

        for stack in stacks:
            for index, name in enumerate(stack):
                if part in name:
                    caller = next(
                        (above for above in stack[index + 1 :] if "asnkitrt" in above),
                        "?",
                    )
                    counts[caller] += 1

                    break

        show(f"the runtime calling '{part}'", counts, 10)

    for part in frames_of:
        counts = collections.Counter()

        for stack in stacks:
            for index, name in enumerate(stack):
                if part in name:
                    counts[
                        " <- ".join(above[:60] for above in stack[index : index + 4])
                    ] += 1

                    break

        show(f"stacks of '{part}'", counts, 8)


def run(arguments):
    if platform.system() != "Darwin" or not shutil.which("xctrace"):
        sys.exit("run needs xctrace, on macOS; elsewhere, profile it and use report.")

    subprocess.run(
        [
            "cargo",
            "build",
            "--profile",
            "profiling",
            "-p",
            "asnkitrt",
            "--example",
            "profile",
        ],
        cwd=ROOT,
        check=True,
    )

    calls = str(arguments.calls or CALLS[arguments.operation])
    command = [str(BINARY), arguments.codec, arguments.operation, calls]
    name = f"{arguments.codec}-{arguments.operation}"
    trace = OUT / f"{name}.trace"
    export = OUT / f"{name}.xml"

    OUT.mkdir(parents=True, exist_ok=True)
    shutil.rmtree(trace, ignore_errors=True)
    subprocess.run(
        [
            "xctrace",
            "record",
            "--template",
            "Time Profiler",
            "--output",
            str(trace),
            "--launch",
            "--",
            *command,
        ],
        check=True,
        stdout=subprocess.DEVNULL,
    )

    with export.open("w") as file:
        subprocess.run(
            [
                "xctrace",
                "export",
                "--input",
                str(trace),
                "--xpath",
                '/trace-toc/run[@number="1"]/data/table[@schema="time-profile"]',
            ],
            check=True,
            stdout=file,
            stderr=subprocess.DEVNULL,
        )

    # The time and the allocations of a call, without the profiler.
    subprocess.run(
        [*command[:-1], str(min(int(calls), 100_000)), "--allocs"], check=True
    )
    report(xctrace_stacks(export), arguments.top, arguments.callers, arguments.stacks)


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    commands = parser.add_subparsers(dest="command", required=True)

    for command in ("run", "report"):
        sub = commands.add_parser(command)

        if command == "run":
            sub.add_argument("codec", choices=["ber", "der", "oer", "per", "uper"])
            sub.add_argument("operation", choices=["encode", "decode"])
            sub.add_argument("--calls", type=int, help="how many calls to profile")
        else:
            sub.add_argument(
                "file", help="an export of xctrace (.xml) or folded stacks"
            )

        sub.add_argument(
            "--top", type=int, default=25, help="how many functions to show"
        )
        sub.add_argument("--callers", action="append", default=[], metavar="NAME")
        sub.add_argument("--stacks", action="append", default=[], metavar="NAME")

    arguments = parser.parse_args()

    if arguments.command == "run":
        run(arguments)
    else:
        path = arguments.file
        stacks = xctrace_stacks(path) if path.endswith(".xml") else folded_stacks(path)

        report(stacks, arguments.top, arguments.callers, arguments.stacks)


if __name__ == "__main__":
    main()
