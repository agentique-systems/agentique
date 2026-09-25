"""Bounded, read-only accepted publication search; never issues publications."""
import json
import pathlib
import subprocess
import time

ROOT = pathlib.Path(__file__).resolve().parents[2]
records = []

def run(command, timeout=60):
    started = time.monotonic()
    try:
        result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True,
                                errors="replace", timeout=timeout)
        record = dict(command=command, exit_code=result.returncode,
                      output=result.stdout, stderr=result.stderr)
    except subprocess.TimeoutExpired as error:
        record = dict(command=command, exit_code=None, timeout_seconds=timeout,
                      output=str(error.stdout or ""), stderr=str(error.stderr or ""))
    record["elapsed_ms"] = round((time.monotonic()-started)*1000)
    records.append(record)
    print(json.dumps(record), flush=True)

run(["gh", "api", "repos/agentique-systems/agentique/releases", "--jq",
     "map({tag_name,assets:[.assets[]|{name,size}]})"])
run(["gh", "api", "repos/agentique-systems/agentique/actions/artifacts?per_page=100", "--jq",
     "{total_count,artifacts:[.artifacts[]|{id,name,size_in_bytes,expired,created_at}]}"])
run(["git", "worktree", "list", "--porcelain"])
run(["git", "rev-list", "--objects", "--all", "--", "*.agq-runtime", "*canonical.publication.zip", "*kerml.cache", "*systems.cache"])
roots = ["C:/Users/phili/github", "C:/Users/phili/Downloads", "C:/Users/phili/Documents",
         "C:/Users/phili/Desktop", "C:/Users/phili/OneDrive", "C:/Users/phili/.cache",
         "C:/Users/phili/.local", "C:/home", "C:/Sandbox"]
existing = [root for root in roots if pathlib.Path(root).exists()]
run(["rg", "--files", "--hidden", "--no-ignore", "-g", "*.agq-runtime", "-g",
     "*publication*.zip", "-g", "*kerml*.cache", "-g", "*systems*.cache", "-g",
     "*sysml*.cache", "-g", "!node_modules", "-g", "!target", *existing])
records.append(dict(runtime_store_exists=pathlib.Path("C:/Users/phili/.agentique").exists()))
(ROOT / "verification/native-studio/runtime-recovery-commands.json").write_text(
    json.dumps(records, indent=2), encoding="utf-8")
