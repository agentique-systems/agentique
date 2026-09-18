import fs from "node:fs";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
const directory = "verification/language-core-completion-v3";
const capture = JSON.parse(fs.readFileSync(`${directory}/preservation-inputs.json`));
const hash = bytes => createHash("sha256").update(bytes).digest("hex");
for (const [path, expected] of Object.entries(capture.files)) {
  if (hash(fs.readFileSync(path)) !== expected) throw new Error(`Protected bytes changed: ${path}`);
}
const original = fs.readFileSync(`${directory}/original-foundation-review.md`);
if (!fs.readFileSync("docs/language-core-foundation-review.md").subarray(0, original.length).equals(original)) throw new Error("Foundation review history was rewritten");
const previous = JSON.parse(execFileSync("git", ["show", `${capture.base}:standards/baseline-anomalies.json`], { windowsHide: true }));
const current = JSON.parse(fs.readFileSync("standards/baseline-anomalies.json"));
for (const entry of previous.entries) {
  if (!current.entries.some(value => Object.entries(entry).every(([key, old]) => JSON.stringify(value[key]) === JSON.stringify(old)))) throw new Error("Historical anomaly fields changed");
}
console.log(JSON.stringify({ base: capture.base, protectedFiles: Object.keys(capture.files).length, historicalAnomaliesPreserved: previous.entries.length, reviewHistoryPreserved: true }, null, 2));
