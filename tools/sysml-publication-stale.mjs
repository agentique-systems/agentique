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

// This is a fail-closed recognizer of the finite compiled authority table,
// not a general Rust evaluator. Comments, strings and nested test modules
// cannot impersonate a top-level catalogue declaration or an entry field.
function compiledSystemsAuthority(root) {
  const source = fs.readFileSync(
    safePath(root, "crates/kerml-semantics/src/trusted_publication.rs"),
    "utf8",
  );
  const tokens = [];
  for (let at = 0; at < source.length; ) {
    if (/\s/.test(source[at])) {
      at++;
      continue;
    }
    if (source.startsWith("//", at)) {
      const end = source.indexOf("\n", at);
      at = end < 0 ? source.length : end + 1;
      continue;
    }
    if (source.startsWith("/*", at)) {
      let depth = 1;
      at += 2;
      while (depth && at < source.length) {
        if (source.startsWith("/*", at)) {
          depth++;
          at += 2;
        } else if (source.startsWith("*/", at)) {
          depth--;
          at += 2;
        } else at++;
      }
      assert.equal(depth, 0, "unclosed catalogue source comment");
      continue;
    }
    const raw = /^r(#{0,255})"/.exec(source.slice(at));
    if (raw) {
      const start = at + raw[0].length;
      const end = source.indexOf(`"${raw[1]}`, start);
      assert(end >= 0, "unclosed catalogue source raw string");
      tokens.push({ string: source.slice(start, end) });
      at = end + raw[1].length + 1;
      continue;
    }
    if (source[at] === '"') {
      const start = at++;
      while (at < source.length && source[at] !== '"') {
        at += source[at] === "\\" ? 2 : 1;
      }
      assert(at < source.length, "unclosed catalogue source string");
      const text = source.slice(start, ++at);
      // The catalogue identity must be an ordinary literal. Unsupported Rust
      // escape syntax cannot silently produce an absent-authority decision.
      let value;
      try {
        value = JSON.parse(text);
      } catch {
        value = undefined;
      }
      tokens.push({ string: value });
      continue;
    }
    const identifier = /^[A-Za-z_][A-Za-z_0-9]*/.exec(source.slice(at));
    tokens.push(identifier ? identifier[0] : source[at]);
    at += identifier ? identifier[0].length : 1;
  }
  let depth = 0;
  for (let index = 0; index < tokens.length; index++) {
    const token = tokens[index];
    if (token === "{") depth++;
    else if (token === "}") depth--;
    if (depth !== 0 || token !== "const" || tokens[index + 1] !== "CATALOGUE")
      continue;
    const equals = tokens.indexOf("=", index + 2);
    const end = tokens.indexOf(";", equals + 1);
    assert(
      equals > index &&
        end > equals &&
        tokens[equals + 1] === "&" &&
        tokens[equals + 2] === "[" &&
        tokens[end - 1] === "]",
      "unsupported compiled publication catalogue; authority presence must be reviewed",
    );
    const entries = tokens.slice(equals + 3, end - 1);
    const ids = [];
    for (let entry = 0; entry < entries.length; ) {
      assert(
        entries[entry++] === "CatalogueEntry" && entries[entry++] === "{",
        "unrecognized compiled publication catalogue entry",
      );
      let braces = 1,
        id;
      while (entry < entries.length && braces > 0) {
        const field = entries[entry++];
        if (field === "{") braces++;
        else if (field === "}") braces--;
        if (braces === 1 && field === "id" && entries[entry] === ":") {
          assert.equal(
            id,
            undefined,
            "duplicate compiled publication identity",
          );
          id = entries[entry + 1]?.string;
          assert.equal(
            typeof id,
            "string",
            "unsupported compiled publication identity",
          );
        }
      }
      assert(
        braces === 0 && typeof id === "string",
        "incomplete compiled publication catalogue entry",
      );
      ids.push(id);
      if (entry < entries.length)
        assert.equal(
          entries[entry++],
          ",",
          "compiled catalogue entry separator",
        );
    }
    return ids.includes("sysml-systems-operational-v2");
  }
  throw new Error("compiled publication catalogue declaration is missing");
}

export function verifySystemsPublicationFreshness(root) {
  const exists = [receiptPath, bindingsPath, inputsPath].map((file) =>
    fs.existsSync(safePath(root, file)),
  );
  if (exists.every((value) => !value)) {
    assert(
      !compiledSystemsAuthority(root),
      "compiled Systems authority requires its accepted receipt, bindings and interpretation inputs",
    );
    return { status: "not-accepted" };
  }
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
