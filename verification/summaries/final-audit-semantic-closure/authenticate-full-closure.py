"""Record the new full checkpoint's transport authentication before finalization.

The independent pin below is retained from the completed producer command's
stdout. This report cannot accept a publication; the Rust direct finalizer must
authenticate its semantic contract and perform every publication audit.
"""
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "verification/scripts"))
from frontier_artifact_evidence import checkpoint_evidence, require

PIN = "15356f167c1b5f9a4095d9a02bcda63a5710a2db2c47c93570de3a0136844f3c"


def read(path):
    raw = (ROOT / path).read_bytes()
    return json.loads(raw), {"path": path, "sha256": hashlib.sha256(raw).hexdigest()}


report, report_pin = read("verification/generated/final-audit-semantic-closure/full-closure/report.json")
run, run_pin = read("verification/generated/overnight-convergence/semantic-closure-systems-full/result.json")
executable, executable_pin = read("verification/summaries/final-audit-semantic-closure/executable.json")
require(run["exit_code"] == 0 and run["safety_stop"] is None, "completed producer command")
require("--close-only" in run["command"] and "--profile=operational-v3" in run["command"], "explicit full closure command")
require(report["scope"] is None and report["sysml_profile"] == "agentique-sysml-2.0-operational/3", "full v3 scope")
require(report["publication_accepted"] is False and report["publication_attempted"] is False, "unaccepted closure only")
require(report["strict_closure_complete"] is True and report["strict_producer_diagnostics"] == [], "clean strict closure")
closure = report["strict_producer_closure"]
require(closure["fully_closed"] is True and closure["incomplete_pairs"] == 0, "fully closed certificate")
require(closure["closed_pairs"] == closure["applicable_pairs"], "all producer pairs closed")
require(closure["closed_requirements"] == closure["required_requirements"], "all requirements closed")
references = report["mandatory_references"]
require(references["total"] == references["counts"]["complete"] == 1327, "full reference population")
require(references["failures"] == [] and all(value == 0 for key, value in references["counts"].items() if key != "complete"), "clean references")
latest = report["checkpoint_session"]["latest"]
require(latest["sha256"] == PIN, "independently retained journal pin")
log = ROOT / run["output"] / "output.log"
log_raw = log.read_bytes()
require(hashlib.sha256(log_raw).hexdigest() == run["output_sha256"], "completed run output pin")
require(f"sha256={PIN} invocation=3 round=28 stratum=ContextualBindings".encode() in log_raw, "durable checkpoint acknowledgement")
with (ROOT / run["command"][0]).open("rb") as binary:
    require(hashlib.file_digest(binary, "sha256").hexdigest() == executable["sha256"], "pinned executable")
# Bind the strict certificate, not the earlier preparation certificate.
transport = checkpoint_evidence({"checkpoint_session": report["checkpoint_session"], "producer_closure": closure})
result = {
    "format": "agq-full-systems-closure-authentication/1",
    "passed": True,
    "publication_authority": False,
    "semantic_authority": "pending Rust direct-finalizer authentication and audits",
    "independently_retained_journal_sha256": PIN,
    "closed_producer_pairs": closure["closed_pairs"],
    "closed_requirements": closure["closed_requirements"],
    "mandatory_references_complete": 1327,
    "producer_diagnostics": [],
    "checkpoint": transport,
    "evidence": [report_pin, run_pin, executable_pin],
}
target = ROOT / "verification/summaries/final-audit-semantic-closure/full-closure-authentication.json"
target.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
print(f"Full closure transport authenticated: {closure['closed_pairs']} pairs; {closure['closed_requirements']} requirements; publication_authority=false")
