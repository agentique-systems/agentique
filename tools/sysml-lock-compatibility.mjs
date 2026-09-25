// A dependency compatibility proof, never a publication or an input recapture.
import assert from "node:assert/strict";
import fs from "node:fs";
import { hash, safePath } from "./extract.mjs";

export const acceptedLockPath =
  "verification/compatibility/sysml-v3-accepted.Cargo.lock";

const normalized = (bytes) =>
  Buffer.from(bytes.toString("utf8").replaceAll("\r\n", "\n"));
const packageIdentity = (pkg) =>
  JSON.stringify([pkg.name, pkg.version, pkg.source ?? null]);

// Cargo's generated v4 lock syntax is intentionally a small supported subset.
// Reject additional keys/tables/syntax rather than silently excluding an input
// from this proof. Dependency strings resolve to exact package identities.
export function parseCargoLock(text) {
  const packages = [];
  const lines = text.replaceAll("\r\n", "\n").split("\n");
  let version;
  let pkg;
  let dependencies = false;
  const string = (value) => {
    assert(/^"[^"\\\r\n]*"$/.test(value), "unsupported Cargo.lock string");
    return JSON.parse(value);
  };
  for (const raw of lines) {
    const line = raw.trim();
    if (!line || line.startsWith("#")) continue;
    if (dependencies) {
      if (line === "]") {
        dependencies = false;
      } else {
        assert(line.endsWith(","), "unsupported Cargo.lock dependency array");
        pkg.dependencies.push(string(line.slice(0, -1)));
      }
      continue;
    }
    if (line === "[[package]]") {
      assert.equal(version, 4, "unsupported Cargo.lock version");
      pkg = {};
      packages.push(pkg);
      continue;
    }
    if (!pkg) {
      assert.equal(version, undefined, "duplicate Cargo.lock header");
      assert.equal(line, "version = 4", "unsupported Cargo.lock header");
      version = 4;
      continue;
    }
    const field = /^(name|version|source|checksum|dependencies) = (.+)$/.exec(
      line,
    );
    assert(field, `unsupported Cargo.lock package field: ${line}`);
    const [, key, value] = field;
    assert(!Object.hasOwn(pkg, key), `duplicate Cargo.lock ${key}`);
    if (key === "dependencies") {
      assert(["[", "[]"].includes(value), "unsupported dependency syntax");
      pkg.dependencies = [];
      dependencies = value === "[";
    } else {
      pkg[key] = string(value);
    }
  }
  assert.equal(version, 4, "missing Cargo.lock version");
  assert(!dependencies, "unterminated Cargo.lock dependencies");
  assert(packages.length > 0, "empty Cargo.lock packages");
  const identities = new Set();
  for (const record of packages) {
    assert(/^[A-Za-z0-9_-]+$/.test(record.name ?? ""), "invalid package name");
    assert(
      /^\d+\.\d+\.\d+(?:[-+][A-Za-z0-9.+-]+)?$/.test(record.version ?? ""),
      "invalid package version",
    );
    if (record.source !== undefined) {
      assert(
        /^(registry|git)\+\S+$/.test(record.source),
        "unsupported Cargo.lock package source",
      );
      if (record.source.startsWith("registry+"))
        assert(record.checksum, "registry package missing checksum");
    } else {
      assert.equal(record.checksum, undefined, "local package has checksum");
    }
    if (record.checksum !== undefined)
      assert(
        /^[0-9a-f]{64}$/.test(record.checksum),
        "invalid package checksum",
      );
    const id = packageIdentity(record);
    assert(!identities.has(id), `duplicate Cargo.lock package ${id}`);
    identities.add(id);
    record.dependencies ??= [];
    assert.equal(
      new Set(record.dependencies).size,
      record.dependencies.length,
      `duplicate dependency in ${id}`,
    );
  }
  return packages;
}

export function languageLockClosure(text, roots) {
  assert(roots.length > 0, "language closure roots are missing");
  const packages = parseCargoLock(text);
  const byName = new Map();
  for (const pkg of packages) {
    const entries = byName.get(pkg.name) ?? [];
    entries.push(pkg);
    byName.set(pkg.name, entries);
  }
  const resolve = (dependency) => {
    const match = /^([A-Za-z0-9_-]+)(?: (\S+))?(?: \(([^()]+)\))?$/.exec(
      dependency,
    );
    assert(match, `unsupported dependency identity ${dependency}`);
    const [, name, version, source] = match;
    const candidates = (byName.get(name) ?? []).filter(
      (candidate) =>
        (version === undefined || candidate.version === version) &&
        (source === undefined || candidate.source === source),
    );
    assert.equal(
      candidates.length,
      1,
      `missing or ambiguous Cargo.lock dependency ${dependency}`,
    );
    return candidates[0];
  };
  // Validate even unreachable package edges: malformed locks cannot qualify.
  const edges = new Map(
    packages.map((pkg) => [
      packageIdentity(pkg),
      pkg.dependencies.map(resolve).map(packageIdentity).sort(),
    ]),
  );
  const byId = new Map(packages.map((pkg) => [packageIdentity(pkg), pkg]));
  const closure = new Map();
  const visit = (pkg) => {
    const id = packageIdentity(pkg);
    if (closure.has(id)) return;
    const dependencies = edges.get(id);
    assert.equal(
      new Set(dependencies).size,
      dependencies.length,
      `duplicate resolved dependency in ${id}`,
    );
    closure.set(id, {
      name: pkg.name,
      version: pkg.version,
      source: pkg.source ?? null,
      checksum: pkg.checksum ?? null,
      dependencies,
    });
    for (const dependency of dependencies) visit(byId.get(dependency));
  };
  for (const name of roots) {
    const pkg = resolve(name);
    assert.equal(pkg.source, undefined, `language root ${name} is not local`);
    visit(pkg);
  }
  return Object.fromEntries(
    [...closure].sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)),
  );
}

export function verifyLanguageLockCompatibility(root, acceptedHash, roots) {
  assert(
    /^[0-9a-f]{64}$/.test(acceptedHash ?? ""),
    "missing accepted lock hash",
  );
  const baseline = fs.readFileSync(safePath(root, acceptedLockPath));
  // This file is copied verbatim from main, not freshly generated. Its hash is
  // authenticated by the existing immutable interpretation-input ledger.
  assert.equal(
    hash(baseline),
    acceptedHash,
    "unauthenticated accepted lock bytes",
  );
  const current = normalized(fs.readFileSync(safePath(root, "Cargo.lock")));
  const expected = languageLockClosure(baseline.toString("utf8"), roots);
  const actual = languageLockClosure(current.toString("utf8"), roots);
  assert.deepEqual(
    actual,
    expected,
    "stale accepted Systems language dependency closure; package identity, checksum or edges changed",
  );
  return {
    status: "accepted-language-lock-closure-unchanged",
    grants_publication_authority: false,
    accepted_lock_sha256: acceptedHash,
    current_lock_sha256: hash(current),
    reference: acceptedLockPath,
    roots,
    reachable_packages: Object.keys(expected).length,
    closure_sha256: hash(Buffer.from(JSON.stringify(expected))),
  };
}
