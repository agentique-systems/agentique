// Repository freshness evidence is separate from accepted publication authority.
import fs from "node:fs";
import path from "node:path";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { hash, safePath, root as repositoryRoot } from "./extract.mjs";

export const receiptPath = "standards/sysml-accepted-publication.json";
export const bindingsPath = "standards/sysml-standard-bindings.json";
export const inputsPath = "standards/sysml-publication-inputs.json";
const sourceRoots = [
  "kernel",
  "kerml",
  "kerml-semantics",
  "kerml-syntax",
  "kerml-text",
  "sysml",
  "sysml-semantics",
  "standard-libraries",
];
const manifests = [
  "Cargo.lock",
  "standards/kerml-accepted-publication.json",
  "standards/kerml-standard-bindings.json",
  "standards/normative/sysml-2.0/library-set.json",
  "standards/grammar/sysml-2.0-operational-v1.json",
  "standards/sysml-2.0-operational-semantic-v2.json",
];
const canonical = (value) =>
  JSON.stringify(value, (_, v) =>
    v && typeof v === "object" && !Array.isArray(v)
      ? Object.fromEntries(
          Object.entries(v).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)),
        )
      : v,
  );
const textHash = (root, file) =>
  hash(
    Buffer.from(
      fs.readFileSync(safePath(root, file), "utf8").replaceAll("\r\n", "\n"),
    ),
  );
const digest = (value) => {
  assert(
    Array.isArray(value) &&
      value.length === 32 &&
      value.every((v) => Number.isInteger(v) && v >= 0 && v < 256),
    "invalid publication digest",
  );
  return Buffer.from(value).toString("hex");
};

function interpretationFiles(root) {
  const result = [...manifests];
  const visit = (directory) => {
    for (const entry of fs.readdirSync(safePath(root, directory), {
      withFileTypes: true,
    })) {
      const file = `${directory}/${entry.name}`;
      if (entry.isDirectory()) visit(file);
      else if (
        entry.name.endsWith(".rs") &&
        !/(?:^tests?\.rs$|_tests?\.rs$)/.test(entry.name)
      )
        result.push(file);
    }
  };
  for (const crate of sourceRoots) {
    result.push(`crates/${crate}/Cargo.toml`);
    visit(`crates/${crate}/src`);
  }
  for (const file of fs.readdirSync(safePath(root, "standards/grammar"))) {
    if (file.endsWith(".json")) result.push(`standards/grammar/${file}`);
  }
  // Includes descriptor/profile authority beyond the two explicit SysML pins.
  for (const file of fs.readdirSync(safePath(root, "standards"))) {
    if (/^(kerml|sysml)-.*(operational|errata).*\.json$/.test(file))
      result.push(`standards/${file}`);
  }
  return [...new Set(result)].sort();
}

export function publicationDocuments(root) {
  const receipt = JSON.parse(fs.readFileSync(safePath(root, receiptPath)));
  const bindings = JSON.parse(fs.readFileSync(safePath(root, bindingsPath)));
  assert.equal(receipt.format, "agq-sysml-accepted-publication/1");
  assert.equal(receipt.status, "accepted");
  assert.equal(bindings.format, "agq-sysml-accepted-bindings/1");
  assert.equal(
    digest(receipt.binding_manifest_sha256),
    hash(Buffer.from(canonical(bindings))),
    "stale Systems binding manifest",
  );
  const identity = receipt.identity;
  for (const [a, b] of [
    ["publication_digest", "accepted_systems_digest"],
    ["semantic_digest", "semantic_digest"],
    ["accepted_kerml_digest", "accepted_kerml_digest"],
    ["systems_kpar", "systems_kpar"],
    ["systems_source_content_set", "systems_source_content_set"],
    ["operational_profile", "operational_profile"],
    ["rule_set", "rule_set"],
    ["producer_registry_digest", "producer_registry_digest"],
    ["producer_closure_digest", "producer_closure_digest"],
  ]) {
    assert.notEqual(identity[a], undefined, `missing ${a}`);
    assert.deepEqual(identity[a], bindings[b], `stale Systems ${a}`);
  }
  assert.equal(
    identity.operational_profile,
    "agentique-sysml-2.0-operational/2",
  );
  const sourceSet = JSON.parse(
    fs.readFileSync(
      safePath(root, "standards/normative/sysml-2.0/library-set.json"),
    ),
  );
  assert.equal(receipt.source_content_set, sourceSet.id);
  assert.equal(bindings.source_content_set, sourceSet.id);
  assert.equal(
    digest(identity.systems_source_content_set),
    sourceSet.id.replace(/^sha256:/, ""),
  );
  assert.equal(
    identity.systems_kpar,
    sourceSet.artifacts.find((a) => a.specification === "SysML").sha256,
  );
  const kerml = JSON.parse(
    fs.readFileSync(
      safePath(root, "standards/kerml-accepted-publication.json"),
    ),
  );
  assert.equal(kerml.status, "accepted");
  assert.deepEqual(
    identity.accepted_kerml_digest,
    kerml.complete_overlay.identity.semantic_digest,
  );
  for (const [field, file] of [
    [
      "grammar_compatibility_manifest",
      "standards/grammar/sysml-2.0-operational-v1.json",
    ],
    [
      "semantic_correction_manifest",
      "standards/sysml-2.0-operational-semantic-v2.json",
    ],
  ]) {
    assert.equal(
      digest(identity[field]),
      textHash(root, file),
      `stale Systems ${field}`,
    );
  }
  for (const field of [
    "semantic_digest",
    "publication_digest",
    "producer_registry_digest",
    "producer_closure_digest",
    "dependency_contract_digest",
    "combined_descriptor_graph",
    "producer_context_contract_digest",
  ])
    digest(identity[field]);
  assert.equal(bindings.bindings.length, 69);
  assert.equal(new Set(bindings.bindings.map((b) => b.role)).size, 69);
  return { receipt, bindings };
}

export function capturePublicationInputs(root) {
  const { receipt, bindings } = publicationDocuments(root);
  return {
    format: "agq-sysml-publication-inputs/1",
    grants_publication_authority: false,
    publication_identity: receipt.identity,
    receipt_sha256: hash(Buffer.from(canonical(receipt))),
    bindings_sha256: hash(Buffer.from(canonical(bindings))),
    inputs: Object.fromEntries(
      interpretationFiles(root).map((file) => [file, textHash(root, file)]),
    ),
  };
}

export function verifySystemsPublicationFreshness(root) {
  const exists = [receiptPath, bindingsPath, inputsPath].map((file) =>
    fs.existsSync(safePath(root, file)),
  );
  if (exists.every((value) => !value)) return { status: "not-accepted" };
  assert(
    exists.every(Boolean),
    "accepted Systems receipt, bindings and interpretation inputs must exist together",
  );
  const expected = JSON.parse(fs.readFileSync(safePath(root, inputsPath)));
  const actual = capturePublicationInputs(root);
  assert.deepEqual(
    actual,
    expected,
    "stale accepted Systems publication; review changed source/profile/descriptor/registry/binding/semantic identities",
  );
  return {
    status: "accepted-inputs-current",
    publication_digest: digest(actual.publication_identity.publication_digest),
    checked_inputs: Object.keys(actual.inputs).length,
  };
}

if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  if (process.argv.slice(2).join(" ") !== "--capture")
    throw new Error("usage: node tools/sysml-publication-stale.mjs --capture");
  const value = capturePublicationInputs(repositoryRoot);
  fs.writeFileSync(
    safePath(repositoryRoot, inputsPath),
    JSON.stringify(value, null, 2) + "\n",
  );
  console.log(
    "Recorded Systems interpretation input freshness; no publication authority issued.",
  );
}
