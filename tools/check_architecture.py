#!/usr/bin/env python3
"""Check the Cargo crate graph against the SysML architecture model (ROADMAP R-15, R-41).

models/agentique/*.sysml maps every workspace crate to a part definition
(`part 'agq-language' : Crate;` inside `part def LanguageCore`) and lists every allowed
dependency between parts (`dependency from Studio to SystemState;`). A crate may use
crates of its own part and of the parts its part depends on directly. Normal and build
dependencies count; dev-dependencies are ignored.
"""

import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MODEL_DIR = ROOT / "models" / "agentique"
LANGUAGE_CORE = "LanguageCore"  # depends on no other part
CORE_PARTS = ("LanguageCore", "SystemState", "History")  # locked core (R-16)
# Parts that must not use UI or network libraries: the locked core; the Library (C-49),
# which plans System State changes for the Studio and the Assistant; and the parts of
# the factory loop (C-50): runs stay offline by construction, and processes and live
# model calls are reached only through what the Studio gives them.
PLAIN_PARTS = CORE_PARTS + ("Library", "Simulation", "Implementation", "Execution")
DENIED_LIBRARIES = {
    "gpui", "gpui-pre", "gpui-pre-platform", "gpui-base",
    "eframe", "egui", "egui-wgpu", "wgpu", "winit", "slint",
    "reqwest", "hyper", "axum", "tokio", "tower-http", "ureq",
}
PROVIDERS = "Providers"
# rig (every `rig-*` crate), the async runtime, HTTP and the credential store: only
# Providers may use them, so provider churn and network access stay in one part
# (R-21, R-41, ROADMAP §8.7).
PROVIDER_LIBRARIES = {
    "rig", "tokio", "reqwest", "keyring", "keyring-core", "windows-native-keyring-store",
}
# Temporary uses of those libraries outside Providers, each until the work item that
# removes it; the model's doc of the part says so too.
TEMPORARY_LIBRARY_USES = {
    ("agq-assistant", "reqwest"): "W5.7",  # the hand-written Claude client
}

# Whitespace, comments and `doc /* ... */` are skipped; names, words and punctuation kept.
TOKEN = re.compile(r"(\s+|//[^\n]*|/\*.*?\*/|doc\s*/\*.*?\*/)|('[^']*'|\w+|[{};:.~])|(.)", re.S)


class ModelError(Exception):
    """The model uses syntax this check does not understand."""


def tokenize(text, source):
    tokens = []
    for match in TOKEN.finditer(text):
        _, token, bad = match.groups()
        if token or bad:
            line = text.count("\n", 0, match.start()) + 1
            if bad:
                raise ModelError(f"{source}:{line}: unexpected {bad!r}")
            tokens.append((token, f"{source}:{line}"))
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
    """Crates named by `part 'name' : Crate;` inside a part def. Other typed part
    usages (`part studio : Studio;`) describe composition and map no crate."""
    crates = []
    for words, inner, where in body:
        if words[0] != "part" or len(words) != 4 or words[2] != ":" or inner:
            raise ModelError(f"{where}: unsupported in a part def: {' '.join(words)}")
        if words[3] == "Crate":
            crates.append(words[1].strip("'"))
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
                else:
                    raise ModelError(f"{where}: unsupported in a package: {' '.join(words)}")
    for client, supplier in dependencies:
        for name in (client, supplier):
            if name not in parts:
                raise ModelError(f"dependency from {client} to {supplier}: no part def {name}")
    return parts, dependencies


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
    problems += [f"{LANGUAGE_CORE} must not depend on {supplier}"
                 for client, supplier in dependencies if client == LANGUAGE_CORE]
    for crate, uses in sorted(crates.items()):
        part = part_of.get(crate)
        if part is None:
            problems.append(f"{crate} is not mapped to any part")
            continue
        allowed = {part} | {supplier for client, supplier in dependencies if client == part}
        for used in uses:
            if used in part_of and part_of[used] not in allowed:
                problems.append(
                    f"{crate} ({part}) depends on {used} ({part_of[used]}), "
                    f"but the model has no dependency from {part} to {part_of[used]}")
            elif part in PLAIN_PARTS and used in DENIED_LIBRARIES:
                problems.append(f"{crate} ({part}) depends on {used}, a UI or network library")
            elif (provider_library(used) and part != PROVIDERS
                  and (crate, used) not in TEMPORARY_LIBRARY_USES):
                problems.append(f"{crate} ({part}) depends on {used}, which only {PROVIDERS} may use")
    for (crate, used), until in sorted(TEMPORARY_LIBRARY_USES.items()):
        if crate in crates and used not in crates[crate]:
            problems.append(f"the temporary use of {used} by {crate} (until {until}) is gone: "
                            "remove it from TEMPORARY_LIBRARY_USES")
    return problems


def provider_library(name):
    return name in PROVIDER_LIBRARIES or name.startswith("rig-")


def crates_from_metadata(metadata):
    """Map each package of `cargo metadata --no-deps` to its normal and build
    dependencies (a dependency's `kind` is null for normal, else "build" or "dev")."""
    return {package["name"]: sorted({dep["name"] for dep in package["dependencies"] if dep["kind"] != "dev"})
            for package in metadata["packages"]}


def cargo_crates():
    command = ["cargo", "metadata", "--format-version", "1", "--no-deps",
               "--manifest-path", str(ROOT / "Cargo.toml")]
    output = subprocess.run(command, check=True, stdout=subprocess.PIPE, encoding="utf-8").stdout
    return crates_from_metadata(json.loads(output))


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
