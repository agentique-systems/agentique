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
import {
  acceptedLockPath,
  languageLockClosure,
  parseCargoLock,
} from "./sysml-lock-compatibility.mjs";

const canonical = (value) =>
  JSON.stringify(value, (_, v) =>
    v && typeof v === "object" && !Array.isArray(v)
      ? Object.fromEntries(
          Object.entries(v).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)),
        )
      : v,
  );
function fixture(t, version = 2) {
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
  write(`standards/sysml-2.0-operational-semantic-v${version}.json`, manifest);
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
    operational_profile: `agentique-sysml-2.0-operational/${version}`,
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

test("transport content and population require reviewed freshness without changing authority", (t) => {
  const { root, write } = fixture(t, 3);
  const transport = "standards/runtime-transports/fixture.json";
  const authority = [receiptPath, bindingsPath].map((file) =>
    fs.readFileSync(path.join(root, file)),
  );
  write(transport, { fixture: "original" });
  assert.throws(
    () => verifySystemsPublicationFreshness(root),
    /stale accepted Systems/,
  );
  write(inputsPath, capturePublicationInputs(root));
  assert.equal(
    verifySystemsPublicationFreshness(root).status,
    "accepted-inputs-current",
  );
  write(transport, { fixture: "changed" });
  assert.throws(
    () => verifySystemsPublicationFreshness(root),
    /stale accepted Systems/,
  );
  fs.unlinkSync(path.join(root, transport));
  assert.throws(
    () => verifySystemsPublicationFreshness(root),
    /stale accepted Systems/,
  );
  for (const [index, file] of [receiptPath, bindingsPath].entries()) {
    assert.deepEqual(fs.readFileSync(path.join(root, file)), authority[index]);
  }
});

test("v3 freshness authenticates the v3 manifest while preserving v2 fixtures", (t) => {
  const { root, write, receipt, bindings } = fixture(t, 3);
  assert.equal(
    verifySystemsPublicationFreshness(root).status,
    "accepted-inputs-current",
  );
  const v3 = "standards/sysml-2.0-operational-semantic-v3.json";
  assert(v3 in capturePublicationInputs(root).inputs);
  // A matching v2 manifest is not a substitute for the receipt's v3 identity.
  write("standards/sysml-2.0-operational-semantic-v2.json", "{}\n");
  write(v3, "changed v3 interpretation\n");
  assert.throws(
    () => capturePublicationInputs(root),
    /stale Systems semantic_correction_manifest/,
  );
  write(v3, "{}\n");
  for (const profile of ["agentique-sysml-2.0-operational/4", "changed"]) {
    const alteredBindings = { ...bindings, operational_profile: profile };
    const alteredReceipt = structuredClone(receipt);
    alteredReceipt.identity.operational_profile = profile;
    alteredReceipt.binding_manifest_sha256 = [
      ...Buffer.from(hash(Buffer.from(canonical(alteredBindings))), "hex"),
    ];
    write(bindingsPath, alteredBindings);
    write(receiptPath, alteredReceipt);
    assert.throws(
      () => capturePublicationInputs(root),
      /unsupported Systems operational profile/,
    );
  }
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

for (const version of [2, 3])
  test(`deleting all authority artifacts after compiled Systems v${version} activation fails closed`, (t) => {
    const { root, write } = fixture(t, version);
    write(
      "crates/kerml-semantics/src/trusted_publication.rs",
      `
    const CATALOGUE: &[CatalogueEntry<'static>] = &[CatalogueEntry {
      id: "sysml-systems-operational-v${version}",
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

const languageRoots = [
  "kernel",
  "kerml",
  "kerml-semantics",
  "kerml-syntax",
  "kerml-text",
  "sysml",
  "sysml-semantics",
  "standard-libraries",
].map((name) => `agq-${name}`);
const registry = "registry+https://github.com/rust-lang/crates.io-index";
function lockText(packages) {
  return (
    "# Test lock; no publication authority\nversion = 4\n" +
    packages
      .map(
        (pkg) =>
          "\n[[package]]\n" +
          Object.entries(pkg)
            .map(([key, value]) =>
              Array.isArray(value)
                ? `${key} = [\n${value.map((s) => ` ${JSON.stringify(s)},\n`).join("")}]\n`
                : `${key} = ${JSON.stringify(value)}\n`,
            )
            .join(""),
      )
      .join("")
  );
}
function lockFixture(t) {
  const f = fixture(t, 3);
  const packages = [
    ...languageRoots.map((name) => ({
      name,
      version: "0.1.0",
      dependencies: ["semantic-helper"],
    })),
    {
      name: "semantic-helper",
      version: "1.0.0",
      source: registry,
      checksum: "12".repeat(32),
      dependencies: ["leaf"],
    },
    {
      name: "leaf",
      version: "1.0.0",
      source: registry,
      checksum: "34".repeat(32),
    },
    { name: "app", version: "0.1.0" },
  ];
  const baseline = lockText(packages);
  f.write("Cargo.lock", baseline);
  f.write(acceptedLockPath, baseline);
  f.write(inputsPath, capturePublicationInputs(f.root));
  return { ...f, packages, baseline };
}

test("app-only lock additions prove unchanged full language closure without recapturing authority", (t) => {
  const { root, write, packages } = lockFixture(t);
  const authority = fs.readFileSync(path.join(root, inputsPath));
  packages.push({ name: "native-ui", version: "0.1.0", dependencies: ["app"] });
  write("Cargo.lock", lockText(packages));
  const proof = verifySystemsPublicationFreshness(root);
  assert.equal(proof.status, "accepted-inputs-compatible");
  assert.equal(proof.lock_compatibility.reachable_packages, 10);
  assert.equal(proof.lock_compatibility.grants_publication_authority, false);
  assert.notEqual(
    proof.lock_compatibility.current_lock_sha256,
    proof.lock_compatibility.accepted_lock_sha256,
  );
  assert.deepEqual(fs.readFileSync(path.join(root, inputsPath)), authority);
  // Compatibility cannot cover even an unrelated new interpretation file.
  write("crates/sysml/src/new.rs", "// new semantic implementation");
  assert.throws(
    () => verifySystemsPublicationFreshness(root),
    /stale accepted Systems/,
  );
});

test("every reachable package identity, checksum and edge remains immutable", (t) => {
  const { root, write, packages } = lockFixture(t);
  for (const change of [
    (p) => {
      p[8].version = "1.0.1";
    },
    (p) => {
      p[8].source = "registry+https://example.invalid/index";
    },
    (p) => {
      p[8].checksum = "ff".repeat(32);
    },
    (p) => {
      p[8].dependencies = [];
    },
    (p) => {
      p[8].dependencies.push("app");
    },
    (p) => {
      p[0].dependencies.push("app");
    },
    (p) => {
      p[0].name = "renamed-language-root";
    },
  ]) {
    const changed = structuredClone(packages);
    change(changed);
    write("Cargo.lock", lockText(changed));
    assert.throws(() => verifySystemsPublicationFreshness(root));
  }
});

test("duplicate-version lock qualification resolves exact edges and rejects ambiguity", (t) => {
  const { root, write, packages } = lockFixture(t);
  packages.push({
    name: "leaf",
    version: "2.0.0",
    source: registry,
    checksum: "56".repeat(32),
  });
  write("Cargo.lock", lockText(packages));
  assert.throws(() => verifySystemsPublicationFreshness(root), /ambiguous/);
  packages[8].dependencies = ["leaf 1.0.0"];
  write("Cargo.lock", lockText(packages));
  assert.equal(
    verifySystemsPublicationFreshness(root).status,
    "accepted-inputs-compatible",
  );
  packages[8].dependencies = [`leaf 1.0.0 (${registry})`];
  write("Cargo.lock", lockText(packages));
  assert.equal(
    verifySystemsPublicationFreshness(root).status,
    "accepted-inputs-compatible",
  );
  packages[8].dependencies = ["leaf 2.0.0"];
  write("Cargo.lock", lockText(packages));
  assert.throws(
    () => verifySystemsPublicationFreshness(root),
    /language dependency closure/,
  );
});

test("compatibility authenticates original baseline bytes and fails on missing reference", (t) => {
  const { root, write, packages, baseline } = lockFixture(t);
  packages.push({ name: "native-ui", version: "0.1.0" });
  write("Cargo.lock", lockText(packages));
  write(acceptedLockPath, baseline + "\n");
  assert.throws(
    () => verifySystemsPublicationFreshness(root),
    /unauthenticated/,
  );
  fs.unlinkSync(path.join(root, acceptedLockPath));
  assert.throws(() => verifySystemsPublicationFreshness(root), /ENOENT/);
});

test("lock parser rejects unknown fields, tables, duplicates and malformed resolution", (t) => {
  const { packages, baseline } = lockFixture(t);
  for (const text of [
    baseline.replace("version = 4", "version = 3"),
    baseline + "[metadata]\nignored = true\n",
    baseline + 'replace = "hidden-package"\n',
    baseline + 'name = "duplicate-name"\n',
    baseline.replace('checksum = "' + "12".repeat(32) + '"\n', ""),
    lockText([...packages, packages[0]]),
    lockText([
      { ...packages[0], dependencies: ["missing-package"] },
      ...packages.slice(1),
    ]),
    lockText([
      {
        ...packages[0],
        dependencies: ["semantic-helper", "semantic-helper 1.0.0"],
      },
      ...packages.slice(1),
    ]),
  ]) {
    assert.throws(() => languageLockClosure(text, languageRoots));
  }
  assert.equal(parseCargoLock(baseline).length, packages.length);
});
