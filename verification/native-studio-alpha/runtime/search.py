"""Final read-only runtime archaeology. Candidates still require facade authentication."""
import concurrent.futures
import json
import pathlib
import subprocess
import time

ROOT = pathlib.Path(__file__).resolve().parents[3]
DEST = pathlib.Path(__file__).resolve().parent
roots = ["C:/Users/phili/github", "C:/Users/phili/Downloads", "C:/Users/phili/Documents",
         "C:/Users/phili/Desktop", "C:/Users/phili/OneDrive", "C:/Users/phili/.cache",
         "C:/Users/phili/.local", "C:/Users/phili/.agentique", "C:/Users/phili/.codex",
         "C:/home", "C:/Sandbox"]
commands = [
    ["gh", "api", "repos/agentique-systems/agentique/releases", "--jq",
     "map({tag_name,assets:[.assets[]|{id,name,size}]})"],
    ["gh", "api", "--paginate", "repos/agentique-systems/agentique/actions/artifacts?per_page=100"],
    ["gh", "api", "--paginate", "repos/agentique-systems/agentique/actions/caches?per_page=100"],
    ["git", "worktree", "list", "--porcelain"],
    ["git", "rev-list", "--objects", "--all", "--", "*.agq-runtime", "*publication*.zip", "*kerml.cache", "*systems.cache"],
    ["rg", "--files", "--hidden", "--no-ignore", "-g", "*.agq-runtime", "-g",
     "*publication*.zip", "-g", "*kerml*.cache", "-g", "*systems*.cache", "-g", "*agentique*.zip", "-g", "*Agentique*.zip", "-g", "*repo-last*.zip", "-g", "!node_modules", "-g",
     "!target", "-g", "!registry", *[p for p in roots if pathlib.Path(p).exists()]],
]

def run(index_command):
    index, command = index_command
    started = time.monotonic()
    result = subprocess.run(command, cwd=ROOT, capture_output=True, timeout=180)
    output = result.stdout.decode("utf-8", errors="replace")
    stderr = result.stderr.decode("utf-8", errors="replace")
    record = dict(command=command, exit_code=result.returncode, output=output, stderr=stderr,
                  elapsed_seconds=round(time.monotonic()-started, 3))
    (DEST / f"search-{index}.json").write_text(json.dumps(record, indent=2), encoding="utf-8")
    return dict(index=index, exit_code=result.returncode, output_bytes=len(result.stdout))

with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
    print(json.dumps(list(pool.map(run, enumerate(commands))), indent=2))
