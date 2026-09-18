// Verify the immutable inputs and original blocked-run evidence against fetched main.
import fs from "node:fs";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import assert from "node:assert/strict";

const base = "736142f7f887252f83b44ed7c54fe540d138dbaf";
const paths = [
  "standards/normative", "standards/artifacts", "standards/libraries", "standards/libraries-2026-04",
  "standards/coverage.json", "verification/traceability.json", "KerML.pdf", "SysML.pdf", "SysAPI.pdf",
  "Agentique-Specification-v0.1.html", "crates/kerml/src/generated", "standards/generated/kerml-1.0",
  "verification/sysml-language-v2", "verification/modeling-platform-v2", "verification/standards-integrity.json",
  "verification/browser-results.json", "verification/screenshots",
  ...fs.readdirSync("docs/adr").filter((name) => /^000[1-7]-/.test(name)).map((name) => `docs/adr/${name}`),
];
execFileSync("git", ["diff", "--exit-code", base, "--", ...paths], { stdio: "inherit", windowsHide: true });
const originalAudit = execFileSync("git", ["show", `${base}:standards/generated/sysml-2.0/structural-audit.json`], { windowsHide: true });
assert.deepEqual(fs.readFileSync("verification/sysml-language-v2-resume/original-structural-audit.json"), originalAudit);
const originalBlocker = execFileSync("git", ["show", `${base}:docs/sysml-v2-runtime-blocker.md`], { encoding: "utf8", windowsHide: true });
assert(fs.readFileSync("docs/sysml-v2-runtime-blocker.md", "utf8").replaceAll("\r\n", "\n").startsWith(originalBlocker.replaceAll("\r\n", "\n")));
const results = JSON.parse(fs.readFileSync("verification/sysml-language-v2-resume/final/results.json", "utf8"));
assert.equal(results.length, 11);
assert(results.every((r) => r.exitCode === 0));
const gate = JSON.parse(fs.readFileSync("verification/sysml-language-v2-resume/stage-3/results.json", "utf8"));
assert.deepEqual(gate.map((r) => r.exitCode), [1, 0, 0]);
console.log(JSON.stringify({ base, protectedPaths: paths, originalAuditSha256: createHash("sha256").update(originalAudit).digest("hex"), originalBlockerPreserved: true, finalChecks: 11, finalFailures: 0, runtimeGateExitCode: 1 }, null, 2));
