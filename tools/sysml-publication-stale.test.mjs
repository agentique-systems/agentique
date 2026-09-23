import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { hash } from "./extract.mjs";
import {
  receiptPath,
  bindingsPath,
  inputsPath,
  capturePublicationInputs,
  verifySystemsPublicationFreshness,
} from "./sysml-publication-stale.mjs";

const canonical = (value) =>
  JSON.stringify(value, (_, v) =>
    v && typeof v === "object" && !Array.isArray(v)
      ? Object.fromEntries(
          Object.entries(v).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)),
        )
      : v,
  );
function fixture(t) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "agq-systems-stale-"));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const write = (name, value) => {
    fs.mkdirSync(path.dirname(path.join(root, name)), { recursive: true });
    fs.writeFileSync(
      path.join(root, name),
      typeof value === "string" ? value : JSON.stringify(value),
    );
  };
  for (const crate of [
    "kernel",
    "kerml",
    "kerml-semantics",
    "kerml-syntax",
    "kerml-text",
    "sysml",
    "sysml-semantics",
    "standard-libraries",
  ]) {
    write(`crates/${crate}/Cargo.toml`, "fixture manifest");
    write(`crates/${crate}/src/lib.rs`, "// fixture interpretation\n");
  }
  write(
    "crates/kerml-semantics/src/trusted_publication.rs",
    "const CATALOGUE: &[CatalogueEntry<'static>] = &[];\n",
  );
  write("Cargo.lock", "fixture lock");
  const pin = Array(32).fill(1),
    sourcePin = Array(32).fill(2);
  const sourceSet = `sha256:${Buffer.from(sourcePin).toString("hex")}`;
  const kpar = "03".repeat(32);
  write("standards/kerml-accepted-publication.json", {
    status: "accepted",
    complete_overlay: { identity: { semantic_digest: pin } },
  });
  write("standards/kerml-standard-bindings.json", {});
  write("standards/normative/sysml-2.0/library-set.json", {
    id: sourceSet,
    artifacts: [{ specification: "SysML", sha256: kpar }],
  });
  const manifest = "{}\n",
    manifestPin = [...Buffer.from(hash(Buffer.from(manifest)), "hex")];
  write("standards/grammar/sysml-2.0-operational-v1.json", manifest);
  write("standards/sysml-2.0-operational-semantic-v2.json", manifest);
  const identity = Object.fromEntries(
    [
      "publication_digest",
      "semantic_digest",
      "accepted_kerml_digest",
      "producer_registry_digest",
      "producer_closure_digest",
      "dependency_contract_digest",
      "combined_descriptor_graph",
      "producer_context_contract_digest",
    ].map((field) => [field, pin]),
  );
  Object.assign(identity, {
    systems_kpar: kpar,
    systems_source_content_set: sourcePin,
    operational_profile: "agentique-sysml-2.0-operational/2",
    rule_set: "agq-sysml-query/5",
    grammar_compatibility_manifest: manifestPin,
    semantic_correction_manifest: manifestPin,
  });
  const bindings = {
    ...identity,
    format: "agq-sysml-accepted-bindings/1",
    source_content_set: sourceSet,
    accepted_systems_digest: pin,
    bindings: Array.from({ length: 69 }, (_, role) => ({
      role: `fixture-${role}`,
      element: role,
    })),
  };
  const receipt = {
    format: "agq-sysml-accepted-publication/1",
    status: "accepted",
    identity,
    source_content_set: sourceSet,
    binding_manifest_sha256: [
      ...Buffer.from(hash(Buffer.from(canonical(bindings))), "hex"),
    ],
  };
  write(receiptPath, receipt);
  write(bindingsPath, bindings);
  write(inputsPath, capturePublicationInputs(root));
  return { root, write, receipt, bindings };
}

test("accepted Systems freshness binds interpretation population and identities", (t) => {
  const { root, write } = fixture(t);
  assert.equal(
    verifySystemsPublicationFreshness(root).status,
    "accepted-inputs-current",
  );
  write("crates/sysml-semantics/src/new_rule.rs", "// added rule");
  assert.throws(
    () => verifySystemsPublicationFreshness(root),
    /stale accepted Systems/,
  );
});

test("descriptor, registry, profile and source changes fail the repository stale gate", (t) => {
  const { root, write } = fixture(t);
  for (const file of [
    "crates/sysml/src/lib.rs",
    "crates/sysml-semantics/src/lib.rs",
    "standards/sysml-2.0-operational-semantic-v2.json",
    "standards/grammar/sysml-2.0-operational-v1.json",
  ]) {
    const original = fs.readFileSync(path.join(root, file));
    write(file, "changed interpretation");
    assert.throws(() => verifySystemsPublicationFreshness(root));
    fs.writeFileSync(path.join(root, file), original);
  }
  assert.equal(
    verifySystemsPublicationFreshness(root).status,
    "accepted-inputs-current",
  );
});

test("changed receipt, semantic identity or binding cannot retain old freshness", (t) => {
  const { root, write, receipt, bindings } = fixture(t);
  for (const field of [
    "semantic_digest",
    "producer_registry_digest",
    "combined_descriptor_graph",
    "operational_profile",
    "rule_set",
  ]) {
    const changed = structuredClone(receipt);
    changed.identity[field] = Array.isArray(changed.identity[field])
      ? Array(32).fill(9)
      : "changed";
    write(receiptPath, changed);
    assert.throws(() => verifySystemsPublicationFreshness(root));
    write(receiptPath, receipt);
  }
  const changed = structuredClone(bindings);
  changed.bindings[0].element = "changed";
  write(bindingsPath, changed);
  assert.throws(
    () => verifySystemsPublicationFreshness(root),
    /stale Systems binding manifest/,
  );
});

test("partial authority installation fails closed", (t) => {
  const { root } = fixture(t);
  fs.unlinkSync(path.join(root, inputsPath));
  assert.throws(
    () => verifySystemsPublicationFreshness(root),
    /must exist together/,
  );
  fs.unlinkSync(path.join(root, receiptPath));
  fs.unlinkSync(path.join(root, bindingsPath));
  assert.equal(verifySystemsPublicationFreshness(root).status, "not-accepted");
});

test("deleting all authority artifacts after compiled Systems activation fails closed", (t) => {
  const { root, write } = fixture(t);
  write(
    "crates/kerml-semantics/src/trusted_publication.rs",
    `
    const CATALOGUE: &[CatalogueEntry<'static>] = &[CatalogueEntry {
      id: "sysml-systems-operational-v2",
      receipt_format: "agq-sysml-accepted-publication/1",
      receipt: include_str!("../../../standards/sysml-accepted-publication.json"),
      bindings: include_str!("../../../standards/sysml-standard-bindings.json"),
    }];`,
  );
  for (const file of [receiptPath, bindingsPath, inputsPath])
    fs.unlinkSync(path.join(root, file));
  assert.throws(
    () => verifySystemsPublicationFreshness(root),
    /compiled Systems authority requires/,
  );
});

test("comments, strings and test module entries do not activate compiled authority", (t) => {
  const { root, write } = fixture(t);
  write(
    "crates/kerml-semantics/src/trusted_publication.rs",
    `
    // const CATALOGUE: &[CatalogueEntry<'static>] = &[CatalogueEntry { id: "sysml-systems-operational-v2" }];
    /* nested /* catalogue */ comment { id: "sysml-systems-operational-v2" } */
    const TEXT: &str = r#"const CATALOGUE: &[CatalogueEntry<'static>] = &[CatalogueEntry { id: "sysml-systems-operational-v2" }];"#;
    const CATALOGUE: &[CatalogueEntry<'static>] = &[];
    #[cfg(test)] mod tests {
      const CATALOGUE: &[CatalogueEntry<'static>] = &[CatalogueEntry { id: "sysml-systems-operational-v2" }];
    }`,
  );
  for (const file of [receiptPath, bindingsPath, inputsPath])
    fs.unlinkSync(path.join(root, file));
  assert.equal(verifySystemsPublicationFreshness(root).status, "not-accepted");
});

test("unrecognized compiled authority cannot imply a preaccept state", (t) => {
  const { root, write } = fixture(t);
  for (const file of [receiptPath, bindingsPath, inputsPath])
    fs.unlinkSync(path.join(root, file));
  for (const source of [
    "const CATALOGUE: &[CatalogueEntry<'static>] = generated_catalogue!();",
    "#[cfg(test)] mod tests { const CATALOGUE: &[CatalogueEntry<'static>] = &[]; }",
    "const CATALOGUE: &[CatalogueEntry<'static>] = &[SYSTEMS_ENTRY];",
    'const CATALOGUE: &[CatalogueEntry<\'static>] = &[SYSTEMS_ENTRY, CatalogueEntry { id: "other" }];',
  ]) {
    write("crates/kerml-semantics/src/trusted_publication.rs", source);
    assert.throws(() => verifySystemsPublicationFreshness(root));
  }
});
