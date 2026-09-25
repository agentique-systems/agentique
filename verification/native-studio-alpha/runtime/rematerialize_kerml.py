"""Run the exact historical producer without replacing any trusted authority."""
import json
import hashlib
import pathlib
import subprocess
import sys
import time

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[2]
HISTORICAL = ROOT.parent / "agentique-alpha-rematerialize-kerml"
ARTIFACTS = ROOT / "verification/generated/native-studio-alpha/rematerialized-kerml"
ARTIFACTS.mkdir(parents=True, exist_ok=True)
build = HERE / "historical-kerml-recovery-build.json"
while not build.exists():
    time.sleep(1)
if json.loads(build.read_text())["exit_code"] != 0:
    raise SystemExit("Historical producer build failed; see retained build output")
expected = "a1a7b847ef83f8e4c54bea9242cac94a9ce665fe"
actual = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=HISTORICAL, text=True).strip()
if actual != expected or subprocess.check_output(["git", "diff", "HEAD"], cwd=HISTORICAL):
    raise SystemExit("Historical producer source must be the exact unchanged accepted producer")

def run(name, command):
    subprocess.run([sys.executable, str(HERE / "run_record.py"), "--cwd", str(HISTORICAL),
                    "--name", name, "--", *command], check=True)

wrapper = ROOT / "tools/runtime-recovery/kerml_rematerialize.rs"
copied = HISTORICAL / "crates/kerml-text/examples/kerml_rematerialize.rs"
if copied.read_bytes() != wrapper.read_bytes():
    raise SystemExit("Recovery wrapper must match the retained reviewed source")
producer = json.loads((HERE / "rematerialization-producer.json").read_text())
binary = HISTORICAL / "target/release/examples/kerml_rematerialize.exe"
if (producer["producer_commit"] != expected
        or hashlib.sha256(wrapper.read_bytes()).hexdigest() != producer["wrapper_sha256"]
        or hashlib.sha256(binary.read_bytes()).hexdigest() != producer["wrapper_binary_sha256"]):
    raise SystemExit("Recovery source or executable differs from retained producer provenance")
run("historical-kerml-rematerialize", [str(binary),
    "--authority-root=" + str(ROOT), "--output=" + str(ARTIFACTS / "canonical.json")])
