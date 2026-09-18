import fs from "node:fs";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";

const base = "76aef699ee7b2d4f910c7483e42340998770ac5c";
const git = args => execFileSync("git", args, { maxBuffer: 100 * 1024 * 1024, windowsHide: true });
const files = git(["ls-tree", "-r", "--name-only", base]).toString().trim().split("\n");
const prefixes = ["verification/", "standards/normative/", "standards/libraries/", "crates/kernel/", "crates/kerml/", "crates/kerml-semantics/", "crates/kerml-syntax/", "crates/kerml-text/"];
const exact = ["KerML.pdf", "SysML.pdf", "SysAPI.pdf", "Agentique-Specification-v0.1.html", "standards/coverage.json", "standards/lock.json", "standards/baseline-lock.json", "requirements.json", "standards/generated/sysml-2.0/structural-audit.json", "standards/generated/sysml-2.0/metamodel.json", "standards/generated/kerml-1.0/metamodel.json", "standards/generated/kerml-1.0/root-core.golden.json"];
let checked = 0;
const hash = bytes => createHash("sha256").update(bytes).digest("hex");
const capturePath = "verification/language-core-completion-v1/preservation-inputs.json";
const capture = process.argv.includes("--capture");
if (capture && fs.existsSync(capturePath)) throw new Error("Preservation input snapshot already exists");
const captured = capture ? {} : JSON.parse(fs.readFileSync(capturePath));
const checkoutLineEndings = [];
for (const path of files.filter(file => exact.includes(file) || prefixes.some(prefix => file.startsWith(prefix)))) {
  const original = git(["show", `${base}:${path}`]);
  const current = fs.readFileSync(path);
  if (hash(original) !== hash(current)) {
    // Some untouched older reports were already CRLF in this clean checkout,
    // while Git stores LF. Record this distinction; never rewrite their bytes.
    if (!path.startsWith("verification/") || !/\.(md|txt|json|mjs|ps1)$/.test(path) ||
        !original.equals(Buffer.from(current.toString("utf8").replaceAll("\r\n", "\n")))) {
      throw new Error(`Protected content changed: ${path}`);
    }
    checkoutLineEndings.push(path);
  }
  if (capture) captured[path] = hash(current);
  else if (captured[path] !== hash(current)) throw new Error(`Pre-verification protected bytes changed: ${path}`);
  checked++;
}
if (capture) fs.writeFileSync(capturePath, JSON.stringify(captured, null, 2) + "\n");
const previous = JSON.parse(git(["show", `${base}:standards/baseline-anomalies.json`]));
const current = JSON.parse(fs.readFileSync("standards/baseline-anomalies.json"));
for (const entry of previous.entries) {
  if (!current.entries.some(value => JSON.stringify(value) === JSON.stringify(entry))) throw new Error("Historical anomaly disposition changed");
}
if (hash(git(["show", `${base}:standards/generated/sysml-2.0/structural-audit.json`])) !== hash(fs.readFileSync("verification/language-core-completion-v1/original-structural-audit.json"))) throw new Error("Original audit copy differs");
console.log(JSON.stringify({ base, capture, checkedProtectedFiles: checked, checkoutLineEndings, historicalAnomalyEntriesUnchanged: previous.entries.length, originalAuditPreserved: true }, null, 2));
