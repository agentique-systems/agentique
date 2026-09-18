// Inspect the checked-out prerequisite; do not infer runtime support from import coverage.
import fs from "node:fs";
import { execFileSync } from "node:child_process";

const git = (...args) => execFileSync("git", args, { encoding: "utf8" }).trim();
const metadata = JSON.parse(execFileSync("cargo", ["metadata", "--locked", "--offline", "--no-deps", "--format-version", "1"], { encoding: "utf8" }));
const packages = metadata.packages.filter((pkg) => metadata.workspace_members.includes(pkg.id)).map((pkg) => pkg.name).sort();
const required = ["agq-kernel", "agq-kerml", "agq-sysml", "agq-kerml-semantics", "agq-sysml-semantics"];
const missing = required.filter((name) => !packages.includes(name));
const coverage = JSON.parse(fs.readFileSync("standards/v2-coverage.json", "utf8"));
const audit = JSON.parse(fs.readFileSync("standards/generated/sysml-2.0/structural-audit.json", "utf8"));
const blocked = missing.length > 0 || coverage.runtime_sysml_gate?.result !== "passed" || audit.result === "blocked";
console.log(JSON.stringify({
  format: "agentique-platform-prerequisites/1",
  head: git("rev-parse", "HEAD"),
  originMain: git("rev-parse", "origin/main"),
  branch: git("branch", "--show-current"),
  workspacePackages: packages,
  missingRequiredPackages: missing,
  languageRuntimeGate: coverage.runtime_sysml_gate,
  structuralAuditResult: audit.result,
  result: blocked ? "blocked" : "ready-for-review",
  scope: "Necessary prerequisite inventory only; successful inventory would not establish complete language milestone acceptance."
}, null, 2));
process.exitCode = blocked ? 1 : 0;
