// Read-only review evidence. This never writes publication inputs or authority.
import fs from "node:fs";
import path from "node:path";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { isDeepStrictEqual } from "node:util";
import { capturePublicationInputs } from "../../tools/sysml-publication-stale.mjs";
import { verifyLanguageLockCompatibility } from "../../tools/sysml-lock-compatibility.mjs";

const root = process.cwd();
const git = (...args) => execFileSync("git", args, { cwd: root, encoding: "utf8" }).trim();
const hash = (text) => createHash("sha256").update(text.replaceAll("\r\n", "\n")).digest("hex");
const manifest = "standards/sysml-publication-inputs.json";
const original = fs.readFileSync(path.join(root, manifest), "utf8");
const before = JSON.parse(original);
const head = git("rev-parse", "HEAD");
const baseline = git("rev-parse", "origin/main");
const actual = capturePublicationInputs(root);
const roots = ["kernel", "kerml", "kerml-semantics", "kerml-syntax", "kerml-text", "sysml", "sysml-semantics", "standard-libraries"];
let lockCompatibility = null;
if (actual.inputs["Cargo.lock"] !== before.inputs["Cargo.lock"]) {
  lockCompatibility = verifyLanguageLockCompatibility(root, before.inputs["Cargo.lock"], roots.map((name) => `agq-${name}`));
  actual.inputs["Cargo.lock"] = before.inputs["Cargo.lock"];
}
const entries = [...new Set([...Object.keys(before.inputs), ...Object.keys(actual.inputs)])].sort();
const changes = entries.filter((file) => before.inputs[file] !== actual.inputs[file]).map((file) => ({
  file,
  kind: !(file in before.inputs) ? "add" : !(file in actual.inputs) ? "remove" : "update",
  before: before.inputs[file] ?? null,
  after: actual.inputs[file] ?? null,
}));
const fields = (value) => Object.fromEntries(Object.entries(value).filter(([key]) => key !== "inputs"));
const changedFiles = git("diff", "--name-only", "origin/main", "--", ...roots.map((name) => `crates/${name}`)).split("\n").filter(Boolean);
const protectedFiles = [
  "standards/kerml-accepted-publication.json",
  "standards/kerml-standard-bindings.json",
  "standards/sysml-accepted-publication.json",
  "standards/sysml-standard-bindings.json",
  "standards/runtime-transports/sysml-v3-rematerialized-2026-09-25.json",
  "standards/normative/sysml-2.0/library-set.json",
];
const protectedInputs = protectedFiles.map((file) => ({
  file,
  normalized_sha256: hash(fs.readFileSync(path.join(root, file), "utf8")),
  unchanged_from_origin_main: hash(execFileSync("git", ["show", `origin/main:${file}`], { cwd: root, encoding: "utf8" })) === hash(fs.readFileSync(path.join(root, file), "utf8")),
}));
const digest = (value) => Buffer.from(value).toString("hex");
console.log(JSON.stringify({
  format: "agentique-native-alpha-freshness-review/1",
  status: "review-only-no-pin-edits-no-semantic-acceptance",
  grants_publication_authority: false,
  observed_at: new Date().toISOString(),
  head,
  head_after: git("rev-parse", "HEAD"),
  baseline,
  manifest,
  manifest_normalized_sha256: hash(original),
  manifest_unchanged_during_probe: fs.readFileSync(path.join(root, manifest), "utf8") === original,
  previous_input_count: Object.keys(before.inputs).length,
  current_input_count: Object.keys(actual.inputs).length,
  authority_and_non_input_fields_unchanged: isDeepStrictEqual(fields(before), fields(actual)),
  changes,
  changed_language_files: changedFiles.map((file) => ({
    file,
    captured_now: file in actual.inputs,
    previously_captured: file in before.inputs,
  })),
  protected_inputs: protectedInputs,
  accepted_identity: {
    profile: before.publication_identity.operational_profile,
    publication_digest: digest(before.publication_identity.publication_digest),
    semantic_digest: digest(before.publication_identity.semantic_digest),
    accepted_kerml_digest: digest(before.publication_identity.accepted_kerml_digest),
    producer_closure_digest: digest(before.publication_identity.producer_closure_digest),
    systems_kpar: before.publication_identity.systems_kpar,
    systems_source_content_set: digest(before.publication_identity.systems_source_content_set),
  },
  lock_compatibility: lockCompatibility,
}, null, 2));
