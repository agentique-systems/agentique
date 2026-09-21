"""Run required workspace verification; retain raw output only in ignored storage."""
import argparse
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
SUMMARY = "verification/kerml-canonical-publication/summary.json"
RUST = [
    ("fmt", ["cargo", "fmt", "--all", "--", "--check"]),
    ("clippy", ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]),
    ("workspace-tests", ["cargo", "test", "--workspace"]),
    ("metamodel", ["cargo", "run", "--locked", "--offline", "-p", "agq-metamodel-gen", "--", "--check"]),
    ("kerml-runtime", ["cargo", "run", "--locked", "--offline", "-p", "agq-metamodel-gen", "--", "--baseline", "kerml-1.0", "--require-runtime", "--check"]),
    ("sysml-runtime", ["cargo", "run", "--locked", "--offline", "-p", "agq-metamodel-gen", "--", "--baseline", "sysml-2.0", "--require-runtime", "--check"]),
]
NPM = [("standards", ["npm", "run", "standards:check"]),
       ("console-check", ["npm", "run", "check"]),
       ("console-build", ["npm", "run", "build"]),
       ("console-tests", ["npm", "test"]),
       ("e2e", ["npm", "run", "test:e2e"])]


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("scope", choices=["rust", "npm", "all"])
    args = parser.parse_args()
    commands = (RUST if args.scope in ["rust", "all"] else []) + (NPM if args.scope in ["npm", "all"] else [])
    failures = []
    for name, command in commands:
        environment = (["--env", "CARGO_INCREMENTAL=0", "--env", "CARGO_BUILD_JOBS=2"]
                       if command[0] == "cargo" else [])
        code = subprocess.call([sys.executable, "verification/scripts/run.py", "--name", "final-" + name,
                                "--summary", SUMMARY, *environment, "--", *command], cwd=ROOT)
        if code:
            failures.append(name)
    if args.scope in ["rust", "all"]:
        command = ["cargo", "doc", "--locked", "--offline", "--no-deps"]
        for crate in ["agq-kernel", "agq-kerml", "agq-kerml-semantics", "agq-kerml-syntax", "agq-kerml-text", "agq-sysml"]:
            command += ["-p", crate]
        code = subprocess.call([sys.executable, "verification/scripts/run.py", "--name", "final-rustdoc",
                                "--summary", SUMMARY, "--env", "RUSTDOCFLAGS=-D warnings",
                                "--env", "CARGO_INCREMENTAL=0", "--env", "CARGO_BUILD_JOBS=2", "--", *command], cwd=ROOT)
        if code:
            failures.append("rustdoc")
    print("Verification failures:", failures, flush=True)
    return int(bool(failures))


if __name__ == "__main__":
    raise SystemExit(main())
