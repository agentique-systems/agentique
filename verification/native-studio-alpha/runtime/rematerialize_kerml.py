"""Run the exact historical producer without replacing any trusted authority."""
import json
import pathlib
import subprocess
import sys
import time

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[2]
HISTORICAL = ROOT.parent / "agentique-alpha-rematerialize-kerml"
ARTIFACTS = ROOT / "verification/generated/native-studio-alpha/rematerialized-kerml"
ARTIFACTS.mkdir(parents=True, exist_ok=True)
build = HERE / "historical-kerml-build.json"
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

run("historical-kerml-slices", [str(HISTORICAL / "target/release/examples/publication_slices.exe"),
    "--slice=all", "--output=" + str(ARTIFACTS / "slices/slice-{slice}.json")])
run("historical-kerml-canonical", [str(HISTORICAL / "target/release/examples/canonical_publication.exe"),
    "--slice-evidence=" + str(ARTIFACTS / "slices"), "--output=" + str(ARTIFACTS / "canonical.json")])
