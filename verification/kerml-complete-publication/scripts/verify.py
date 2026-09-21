"""Required checks; concise command records and ignored raw logs (ADR 0021)."""
import argparse
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
SUMMARY = "verification/kerml-complete-publication/summary.json"
RUST = [
    ("fmt", ["cargo", "fmt", "--all", "--", "--check"]),
    ("clippy", ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]),
    ("workspace-tests", ["cargo", "test", "--workspace"]),
    ("metamodel", ["cargo", "run", "--locked", "--offline", "-p", "agq-metamodel-gen", "--", "--check"]),
    ("kerml-runtime", ["cargo", "run", "--locked", "--offline", "-p", "agq-metamodel-gen", "--", "--baseline", "kerml-1.0", "--require-runtime", "--check"]),
    ("sysml-runtime", ["cargo", "run", "--locked", "--offline", "-p", "agq-metamodel-gen", "--", "--baseline", "sysml-2.0", "--require-runtime", "--check"]),
    ("rustdoc", ["cargo", "doc", "--locked", "--offline", "--no-deps", *sum((["-p", name] for name in [
        "agq-kernel", "agq-kerml", "agq-kerml-semantics", "agq-kerml-syntax", "agq-kerml-text", "agq-sysml"]), [])]),
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
    failed = []
    for name, command in commands:
        environment = sum((["--env", value] for value in ["CARGO_BUILD_JOBS=2", "CARGO_PROFILE_DEV_DEBUG=0", "CARGO_PROFILE_TEST_DEBUG=0", "CARGO_INCREMENTAL=0"]), []) if command[0] == "cargo" else []
        if name == "rustdoc":
            environment += ["--env", "RUSTDOCFLAGS=-D warnings"]
        code = subprocess.call([sys.executable, "verification/scripts/run.py", "--name", "final-" + name,
                                "--summary", SUMMARY, *environment, "--", *command], cwd=ROOT)
        if code:
            failed.append(name)
    print("Failed required checks:", failed, flush=True)
    return int(bool(failed))


if __name__ == "__main__":
    raise SystemExit(main())
