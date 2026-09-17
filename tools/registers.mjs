import fs from "node:fs";
import path from "node:path";
import { root, hash } from "./extract.mjs";
const read = (file) =>
  JSON.parse(
    fs.readFileSync(path.join(root, file), "utf8").replace(/^\uFEFF/, ""),
  );
const definitions = read("requirements.json");
const map = {
  STD01: [
    "crates/semantics/src/lib.rs",
    [
      "crates/semantics/src/lib.rs#required_implicit_relationships_have_real_targets",
    ],
    "verification/independent-validation.json",
    "incomplete",
    "Required inherited/implicit semantic closure and complete connector feature-chain validation remain partial; see standards/coverage.json. Official pilot validation of all starter models and independent fixture passes against corrected, pinned 2026-04 libraries.",
  ],
  STD02: [
    "crates/semantics/src/lib.rs",
    ["crates/simulation/tests/contract.rs#at_std02_parallel_preserved"],
    "verification/results.json",
  ],
  SELF01: [
    "models/AgentiqueArchitecture.sysml",
    [
      "crates/application/tests/traceability.rs#at_self01_trace01_registered_declarations",
    ],
    "verification/results.json",
  ],
  MOD01: [
    "crates/workspace/src/lib.rs",
    [
      "crates/workspace/tests/editing.rs#at_mod01_rename_move_identity",
      "adapters/storage/tests/reliability.rs#completed_runs_survive_restart_and_rename",
    ],
    "verification/results.json",
  ],
  MOD02: [
    "crates/application/src/lib.rs",
    [
      "crates/workspace/tests/editing.rs#at_mod02_atomic_candidate_and_conflict",
      "adapters/storage/tests/reliability.rs#invalid_draft_recoverable",
    ],
    "verification/results.json",
  ],
  MOD03: [
    "crates/workspace/src/lib.rs",
    [
      "crates/workspace/tests/editing.rs#at_mod03_text_and_kpar",
      "crates/workspace/tests/editing.rs#standard_text_identity_sidecar_and_implicit_ids_roundtrip",
    ],
    "verification/results.json",
  ],
  SIM01: [
    "crates/simulation/src/lib.rs",
    [
      "crates/simulation/tests/contract.rs#at_sim01_resolved_manifest",
      "crates/simulation/tests/contract.rs#missing_and_mismatched_bindings",
    ],
    "verification/results.json",
  ],
  SIM02: [
    "crates/simulation/src/lib.rs",
    ["crates/simulation/tests/contract.rs#at_sim02_isolation"],
    "verification/results.json",
  ],
  SIM03: [
    "crates/simulation/src/lib.rs",
    [
      "crates/simulation/tests/contract.rs#at_sim03_sim04_manual_continuous_repeat",
      "crates/simulation/tests/contract.rs#control_state_machine",
    ],
    "verification/demo.json",
  ],
  SIM04: [
    "crates/simulation/src/lib.rs",
    [
      "crates/simulation/tests/contract.rs#at_sim03_sim04_manual_continuous_repeat",
    ],
    "verification/demo.json",
  ],
  SIM05: [
    "crates/simulation/src/lib.rs",
    [
      "crates/simulation/tests/contract.rs#at_sim05_completion_is_not_verdict",
      "crates/simulation/tests/contract.rs#resource_limits_explicit",
    ],
    "verification/demo.json",
  ],
  SIM06: [
    "crates/simulation/src/lib.rs",
    [
      "crates/simulation/tests/contract.rs#at_sim06_source_trace",
      "tests/browser/console.spec.ts#Console lifecycle, source navigation, shared context and reviewed Assistant change",
    ],
    "verification/browser-results.json",
  ],
  UI01: [
    "console/src/main.tsx",
    [
      "tests/browser/console.spec.ts#Console lifecycle, source navigation, shared context and reviewed Assistant change",
    ],
    "verification/browser-results.json",
  ],
  UI02: [
    "console/src/main.tsx",
    [
      "tests/browser/console.spec.ts#Keyboard inspection and recoverable source drafts",
    ],
    "verification/browser-results.json",
  ],
  AI01: [
    "adapters/assistant/src/lib.rs",
    [
      "adapters/storage/tests/reliability.rs#at_ai01_approval_actor_payload_expiry_stale",
      "adapters/storage/tests/reliability.rs#approval_expiry_and_single_use",
      "adapters/assistant/tests/provider.rs#live_transport_uses_typed_real_tools_and_rejects_forgery",
    ],
    "verification/results.json",
  ],
  SEC01: [
    "crates/simulation/src/lib.rs",
    [
      "crates/simulation/tests/contract.rs#at_sec01_live_action_rejected",
      "tests/browser/console.spec.ts#API authentication, scope, identity mapping and revision checks",
    ],
    "verification/results.json",
  ],
  NFR01: [
    "adapters/storage/src/lib.rs",
    [
      "adapters/storage/tests/reliability.rs#failed_durable_commit_not_acknowledged",
      "tools/process-test.mjs#forced process termination",
    ],
    "verification/process-recovery.json",
  ],
  NFR02: [
    "crates/application/src/lib.rs",
    [
      "crates/simulation/tests/contract.rs#resource_limits_explicit",
      "crates/cli/examples/benchmark.rs#fn main",
      "crates/application/tests/cancellation.rs#cancel_during_parsing_preserves_accepted_revision_and_receipts",
      "tests/browser/console.spec.ts#Cancellable model work and paged history use durable Engine results",
    ],
    "verification/benchmark.json",
  ],
  EXT01: [
    "crates/application/src/lib.rs",
    [
      "tools/registers.test.mjs#dependency direction",
      "crates/simulation/tests/contract.rs#independent_model_exact_payload_guard",
    ],
    "verification/results.json",
  ],
  TRACE01: [
    "verification/traceability.json",
    [
      "crates/application/tests/traceability.rs#at_self01_trace01_registered_declarations",
      "tools/registers.test.mjs#every requirement",
    ],
    "verification/results.json",
  ],
};
let verification;
try {
  verification = read("verification/results.json");
} catch {}
const register = {
  format: "agentique-release-traceability/0.1",
  generated_at: new Date().toISOString(),
  baseline_sha256: hash(fs.readFileSync(path.join(root, "requirements.json"))),
  implementation_revision: verification?.source_digest ?? null,
  release_status: "incomplete",
  note: "Passing automated tests establish their stated assertions, not complete standards satisfaction. Incomplete obligations prevent a complete v0.1 release claim.",
  requirements: definitions.map((r) => {
    const [implementation, tests, evidence, status, reason] =
      map[r.id.slice(4)];
    return {
      id: r.id,
      model_ref: r.model_ref,
      model_file: "models/AgentiqueRequirements.sysml",
      verification_ref: `AgentiqueVerification::verify_${r.name}`,
      milestone: r.milestone,
      acceptance_id: r.test_id,
      implementation,
      tests,
      evidence,
      assessment:
        status ??
        (verification?.automated_checks_excluding_documented_validator_disagreement ===
        "pass"
          ? "verified_for_bounded_acceptance"
          : "pending_final_verification"),
      reason:
        reason ??
        "See recorded checks and their explicit scope; no automatic SysML verdict is inferred.",
    };
  }),
};
fs.writeFileSync(
  path.join(root, "verification/traceability.json"),
  JSON.stringify(register, null, 2) + "\n",
);
const api = read("standards/artifacts/OpenAPI.json");
const supported = new Set([
  "/projects",
  "/projects/{projectId}",
  "/projects/{projectId}/branches",
  "/projects/{projectId}/branches/{branchId}",
  "/projects/{projectId}/commits",
  "/projects/{projectId}/commits/{commitId}",
  "/projects/{projectId}/commits/{commitId}/elements",
  "/projects/{projectId}/commits/{commitId}/elements/{elementId}",
  "/projects/{projectId}/commits/{commitId}/roots",
]);
const operations = [];
for (const [route, methods] of Object.entries(api.paths))
  for (const [method, contract] of Object.entries(methods))
    if (["get", "post", "put", "patch", "delete"].includes(method)) {
      const mapped = method === "get" && supported.has(route);
      operations.push({
        method: method.toUpperCase(),
        path: route,
        operation_id: contract.operationId ?? null,
        status: mapped ? "mapped_read_subset" : "not_implemented",
        implementation: mapped ? "crates/server/src/model_api.rs" : null,
        tests: mapped ? ["tests/browser/console.spec.ts"] : [],
        limitations:
          mapped && /elements|roots/.test(route)
            ? "Partial metaclass projection; not full Element interchange schema."
            : "Single local project and main branch.",
      });
    }
fs.writeFileSync(
  path.join(root, "standards/api-coverage.json"),
  JSON.stringify(
    {
      format: "agentique-model-api-coverage/0.1",
      official_openapi_sha256: hash(
        fs.readFileSync(path.join(root, "standards/artifacts/OpenAPI.json")),
      ),
      prefix: "/api/model",
      conformance: "not_claimed",
      official_conformance_suite: "not_run",
      simulation_prefix: "/api/agentique",
      operations,
    },
    null,
    2,
  ) + "\n",
);
console.log(
  `Registered ${register.requirements.length} release obligations and ${operations.length} official API operations.`,
);
