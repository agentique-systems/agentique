#!/usr/bin/env python3
"""Check the Cargo crate graph against the SysML architecture model (REALIGNMENT R-15).

models/agentique/*.sysml maps every workspace crate to a part definition
(`part 'agq-kernel' : Crate;` inside `part def LanguageCore`) and states the allowed
dependencies between parts (`dependency from Studio to SystemState;`). A crate may use
crates of its own part and of every part reachable through those dependencies.
Normal and build dependencies count; dev-dependencies are ignored.
"""

import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MODEL_DIR = ROOT / "models" / "agentique"
# agq-studio-native keeps its own Cargo workspace until it joins the root one.
MANIFESTS = [ROOT / "Cargo.toml", ROOT / "crates" / "studio-native" / "Cargo.toml"]
CORE_PARTS = ("LanguageCore", "SystemState")
DENIED_LIBRARIES = {  # UI, network and AI libraries kept out of the core parts
    "eframe", "egui", "egui-wgpu", "wgpu", "winit",
    "reqwest", "hyper", "axum", "tokio", "tower-http", "ureq",
}
DEFINITIONS = {"item", "port", "interface", "attribute"}  # allowed; the check skips them
USAGES = {"part", "port", "connect"}  # allowed inside a part def

# Whitespace, comments and `doc /* ... */` are skipped; names, words and punctuation kept.
TOKEN = re.compile(r"(\s+|//[^\n]*|/\*.*?\*/|doc\s*/\*.*?\*/)|('[^']*'|\w+|[{};:.~])|(.)", re.S)


class ModelError(Exception):
    """The model uses syntax this check does not understand."""


def tokenize(text, source):
    tokens = []
    for match in TOKEN.finditer(text):
        _, token, bad = match.groups()
        if token or bad:
            where = f"{source}:{text.count(chr(10), 0, match.start()) + 1}"
            if bad:
                raise ModelError(f"{where}: unexpected {bad!r}")
            tokens.append((token, where))
    return tokens


def read_block(tokens, pos=0, nested=False):
    """Read statements up to the closing brace, or to the end at top level.
    A statement is (words, body statements or None, location)."""
    statements, head = [], []
    while pos < len(tokens):
        token, where = tokens[pos]
        pos += 1
        if token == "}":
            if head or not nested:
                raise ModelError(f"{where}: unexpected '}}'")
            return statements, pos
        if token in (";", "{"):
            if not head:
                raise ModelError(f"{where}: unexpected {token!r}")
            body = None
            if token == "{":
                body, pos = read_block(tokens, pos, nested=True)
            statements.append(([word for word, _ in head], body, head[0][1]))
            head = []
        else:
            head.append((token, where))
    if nested or head:
        raise ModelError(f"{tokens[-1][1]}: unexpected end of file")
    return statements, pos


def crates_in(body):
    """Crates named by `part 'name' : Crate;` anywhere inside a part def."""
    crates = []
    for words, inner, where in body:
        if words[0] not in USAGES or "def" in words:
            raise ModelError(f"{where}: unsupported in a part def: {' '.join(words)}")
        if words[0] == "part" and words[2:] == [":", "Crate"]:
            crates.append(words[1].strip("'"))
        crates += crates_in(inner or [])
    return crates


def parse_model(texts):
    """Return ({part def: [crate]}, [(client, supplier)]) from {file name: SysML text}."""
    parts, dependencies = {}, []
    for source, text in sorted(texts.items()):
        packages, _ = read_block(tokenize(text, source))
        for words, body, where in packages:
            if words[0] != "package" or len(words) != 2 or body is None:
                raise ModelError(f"{where}: expected 'package Name {{ ... }}'")
            for words, inner, where in body:
                if words[:2] == ["part", "def"] and len(words) == 3:
                    if words[2] in parts:
                        raise ModelError(f"{where}: part def {words[2]} is defined twice")
                    parts[words[2]] = crates_in(inner or [])
                elif words[0] == "dependency" and len(words) == 5 and words[1::2] == ["from", "to"] and not inner:
                    dependencies.append((words[2], words[4]))
                elif words[0] in DEFINITIONS and words[1:2] == ["def"]:
                    pass
                else:
                    raise ModelError(f"{where}: unsupported in a package: {' '.join(words)}")
    for client, supplier in dependencies:
        for name in (client, supplier):
            if name not in parts:
                raise ModelError(f"dependency from {client} to {supplier}: no part def {name}")
    return parts, dependencies


def reachable(part, dependencies):
    """The part itself and every part it reaches through dependencies."""
    found, todo = {part}, [part]
    while todo:
        current = todo.pop()
        for client, supplier in dependencies:
            if client == current and supplier not in found:
                found.add(supplier)
                todo.append(supplier)
    return found


def check(parts, dependencies, crates):
    """Return one line per problem. `crates` maps each workspace crate to the names of
    its normal and build dependencies."""
    problems, part_of = [], {}
    for part, mapped in parts.items():
        for crate in mapped:
            if crate in part_of:
                problems.append(f"{crate} is mapped to both {part_of[crate]} and {part}")
            part_of.setdefault(crate, part)
            if crate not in crates:
                problems.append(f"{crate} is mapped to {part} but is not a workspace crate")
    problems += [f"the model has no part def {part}" for part in CORE_PARTS if part not in parts]
    for crate, uses in sorted(crates.items()):
        part = part_of.get(crate)
        if part is None:
            problems.append(f"{crate} is not mapped to any part")
            continue
        allowed = reachable(part, dependencies)
        for used in uses:
            if used in part_of and part_of[used] not in allowed:
                problems.append(
                    f"{crate} ({part}) depends on {used} ({part_of[used]}), "
                    f"but the model has no dependency path from {part} to {part_of[used]}")
            elif part in CORE_PARTS and used in DENIED_LIBRARIES:
                problems.append(f"{crate} ({part}) depends on {used}, a UI, network or AI library")
    return problems


def cargo_crates():
    """Map each workspace crate to its normal and build dependencies."""
    crates = {}
    for manifest in MANIFESTS:
        if not manifest.exists():
            continue
        command = ["cargo", "metadata", "--format-version", "1", "--no-deps", "--manifest-path", str(manifest)]
        metadata = json.loads(subprocess.run(command, check=True, stdout=subprocess.PIPE, encoding="utf-8").stdout)
        members = set(metadata["workspace_members"])
        for package in metadata["packages"]:
            if package["id"] in members:
                crates[package["name"]] = sorted({dep["name"] for dep in package["dependencies"] if dep["kind"] != "dev"})
    return crates


def main():
    texts = {path.name: path.read_text(encoding="utf-8") for path in MODEL_DIR.glob("*.sysml")}
    try:
        if not texts:
            raise ModelError(f"no .sysml files in {MODEL_DIR}")
        parts, dependencies = parse_model(texts)
    except ModelError as error:
        print(f"architecture model error: {error}")
        return 1
    crates = cargo_crates()
    problems = check(parts, dependencies, crates)
    for problem in problems:
        print(problem)
    if problems:
        print(f"architecture check failed: {len(problems)} problem(s)")
        return 1
    implemented = sum(1 for mapped in parts.values() if mapped)
    print(f"architecture check OK: {len(crates)} crates in {implemented} parts "
          f"follow {len(dependencies)} allowed dependencies")
    return 0


if __name__ == "__main__":
    sys.exit(main())
