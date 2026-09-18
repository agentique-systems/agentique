"""Preserve/recheck ADR 0010's source facts while independently checking ADR 0011.

The v2 evidence includes hashes of then-blocked full audits. Those historical
hashes must not be rewritten merely because v3 now emits successful full audits.
"""
import importlib.util
import json
from pathlib import Path
import sys

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
sys.dont_write_bytecode=True
spec=importlib.util.spec_from_file_location("authority_v2",HERE.parent/"language-core-completion-v2/authority.py")
previous=importlib.util.module_from_spec(spec)
spec.loader.exec_module(previous)
old=json.loads((HERE.parent/"language-core-completion-v2/evidence/property-authority.json").read_bytes())
current=previous.build()
assert {k:v for k,v in old.items() if k!="complete_audits"}=={k:v for k,v in current.items() if k!="complete_audits"},"ADR 0010 source evidence changed"
audits={name:json.loads((ROOT/f"standards/generated/{name}/full-audit.json").read_bytes()) for name in ("kerml-1.0","sysml-2.0")}
assert all(a["registration_attempted"] and a["registration_error"] is None and a["result"]=="representable" for a in audits.values())
subject="2abb2284-8e25-51ac-b486-1792cc60e1b1"
target="2e4efe58-2d09-5275-991e-104649d59bf3"
diagnostic=next(d for d in audits["sysml-2.0"]["conformance"] if d["subject"]==dict(kind="property",id=subject) and d["rule"]=="RedefinitionContext")
assert diagnostic["related_descriptors"]==[dict(kind="property",id=target)]
assert diagnostic["disposition"]["kind"]=="reviewed-baseline-anomaly"
assert diagnostic["severity"]=="error" and diagnostic["category"]=="conformance"
result=dict(historical_source_evidence_unchanged=True,previous_classification="F",current_classification="E",
            decision="ADR 0011 separates structural representation from authoring conformance; no replacement language semantics invented",
            acceptance_case=current["acceptance_case"],conformance_diagnostic=diagnostic,
            runtime_results={name:a["result"] for name,a in audits.items()})
encoded=(json.dumps(result,sort_keys=True,indent=2)+"\n").encode()
path=HERE/"authority-verification.json"
if "--check" in sys.argv: assert path.read_bytes()==encoded,"v3 authority evidence is stale"
else: path.write_bytes(encoded)
print(encoded.decode())
