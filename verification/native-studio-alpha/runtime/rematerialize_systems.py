"""Continue accepted Systems rematerialization only after exact KerML byte recovery."""
import hashlib
import json
import pathlib
import subprocess
import sys
import time

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[2]
HISTORICAL = ROOT.parent / "agentique-alpha-rematerialize-systems"
TARGET = ROOT.parent / "agentique-alpha-rematerialize-kerml/target/release/examples"
ARTIFACTS = ROOT / "verification/generated/native-studio-alpha"
KERML = ARTIFACTS / "rematerialized-kerml"
SYSTEMS = ARTIFACTS / "rematerialized-systems"

def read_when_complete(path):
    while True:
        try:
            return json.loads(path.read_text())
        except (FileNotFoundError, json.JSONDecodeError):
            for name in ("historical-kerml-build", "historical-kerml-slices", "historical-kerml-canonical"):
                status = HERE / (name + ".json")
                if status.exists() and json.loads(status.read_text())["exit_code"] != 0:
                    raise SystemExit(f"{name} failed; runtime acceptance paused")
            time.sleep(1)

def run(name, command, cwd=HISTORICAL):
    subprocess.run([sys.executable, str(HERE / "run_record.py"), "--cwd", str(cwd),
                    "--name", name, "--", *command], check=True)

build = read_when_complete(HERE / "historical-systems-build.json")
if build["exit_code"] != 0:
    raise SystemExit("Historical Systems build failed")
actual = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=HISTORICAL, text=True).strip()
if actual != "4ac9b8e58695ad837fed6643b0cedfac05d15635" or subprocess.check_output(["git", "diff", "HEAD"], cwd=HISTORICAL):
    raise SystemExit("Systems producer differs from the original accepted producer")
receipt = KERML / "canonical.publication.receipt.json"
read_when_complete(receipt)
run("kerml-exact-transport-recovery", [sys.executable, str(ROOT / "tools/restore-accepted-kerml-transport.py"),
    "--cache", str(KERML / "canonical.publication.zip"), "--generated-receipt", str(receipt),
    "--output", str(KERML / "accepted.cache")], ROOT)
run("historical-systems-closure", [str(TARGET / "sysml_systems_publication.exe"),
    "--cache=" + str(KERML / "accepted.cache"), "--profile=operational-v3", "--close-only",
    "--checkpoint=" + str(SYSTEMS / "frontiers"), "--checkpoint-interval=0",
    "--output=" + str(SYSTEMS / "closure/report.json")])
report = json.loads((SYSTEMS / "closure/report.json").read_text())
latest = report["checkpoint_session"]["latest"]
journal = pathlib.Path(latest["journal"])
actual_digest = hashlib.sha256(journal.read_bytes()).hexdigest()
expected_digest = latest["sha256"]
if not isinstance(expected_digest, str):
    expected_digest = bytes(expected_digest).hex()
if actual_digest != expected_digest:
    raise SystemExit("Retained Systems journal identity mismatch")
(HERE / "rematerialized-systems-journal-pin.json").write_text(json.dumps({
    "journal": str(journal), "sha256": actual_digest}, indent=2))
run("historical-systems-finalizer", [str(TARGET / "sysml_systems_publication.exe"),
    "--cache=" + str(KERML / "accepted.cache"), "--profile=operational-v3",
    "--finalize-converged=" + str(journal), "--resume-sha256=" + actual_digest,
    "--audit-workers=2", "--output=" + str(SYSTEMS / "finalized/report.json")])
