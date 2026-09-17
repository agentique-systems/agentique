import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { spawn, spawnSync } from "node:child_process";
import { root, hash } from "./extract.mjs";
const logDir = path.join(root, "verification/logs");
fs.mkdirSync(logDir, { recursive: true });
const binary = path.join(
  root,
  "target/debug",
  process.platform === "win32" ? "agentique.exe" : "agentique",
);
const demoDb = `.workspaces/verified-demo-${crypto.randomUUID()}.db`;
const commands = [
  ["registers", "node", ["tools/registers.mjs"]],
  ["extraction", "node", ["tools/extract.mjs", "--check"]],
  ["standards", "node", ["tools/standards-check.mjs"]],
  ["rust-format", "cargo", ["fmt", "--all", "--", "--check"]],
  [
    "rust-clippy",
    "cargo",
    [
      "clippy",
      "--workspace",
      "--all-targets",
      "--locked",
      "--",
      "-D",
      "warnings",
    ],
  ],
  ["rust-tests", "cargo", ["test", "--workspace", "--locked"]],
  ["rust-build", "cargo", ["build", "--workspace", "--locked"]],
  ["frontend-format", "npm", ["run", "format:check"]],
  ["frontend-types", "npm", ["run", "check"]],
  ["frontend-build", "npm", ["run", "build"]],
  ["engineering-tests", "npm", ["test"]],
  [
    "independent-starter",
    "node",
    [
      "node_modules/sysml-validate/out/main.js",
      "models/",
      "--library",
      "standards/libraries-2026-04",
      "--no-config",
      "--format",
      "json",
      "--out",
      "verification/independent-validation.json",
    ],
  ],
  [
    "independent-fixtures",
    "node",
    [
      "node_modules/sysml-validate/out/main.js",
      "tests/fixtures/",
      "--library",
      "standards/libraries-2026-04",
      "--no-config",
      "--format",
      "json",
      "--out",
      "verification/independent-fixtures.json",
    ],
  ],
  ["official-pilot", "node", ["tools/pilot-validate.mjs"]],
  ["browser-tests", "npm", ["run", "test:e2e"]],
  ["process-recovery", "node", ["tools/process-test.mjs"]],
  ["headless-demo", binary, ["--workspace", demoDb, "demo"]],
];
const sourceHashes = {};
function sources(dir) {
  for (const entry of fs.readdirSync(path.join(root, dir), {
    withFileTypes: true,
  })) {
    const file = path.posix.join(dir, entry.name);
    if (entry.name === "dist") continue;
    if (entry.isDirectory()) sources(file);
    else sourceHashes[file] = hash(fs.readFileSync(path.join(root, file)));
  }
}
for (const dir of [
  "crates",
  "adapters",
  "console",
  "tools",
  "tests",
  "models",
  "scenarios",
  "contracts",
  "docs",
])
  sources(dir);
for (const f of [
  "Cargo.toml",
  "Cargo.lock",
  "package.json",
  "package-lock.json",
  "rust-toolchain.toml",
  "standards/lock.json",
  "standards/baseline-lock.json",
  "standards/coverage.json",
  "requirements.json",
])
  sourceHashes[f] = hash(fs.readFileSync(path.join(root, f)));
const sourceDigest = hash(
  JSON.stringify(
    Object.fromEntries(
      Object.entries(sourceHashes).sort(([a], [b]) => a.localeCompare(b)),
    ),
  ),
);
const records = [];
async function run(id, executable, args) {
  const start = Date.now();
  console.log(`[${id}] ${executable} ${args.join(" ")}`);
  const windowsNpm = process.platform === "win32" && executable === "npm";
  const child = spawn(
    windowsNpm ? (process.env.ComSpec ?? "cmd.exe") : executable,
    windowsNpm ? ["/d", "/c", "npm.cmd", ...args] : args,
    { cwd: root, windowsHide: true, stdio: ["ignore", "pipe", "pipe"] },
  );
  let output = "",
    stdout = "";
  child.stdout.on("data", (b) => {
    stdout += b;
    output += b;
  });
  child.stderr.on("data", (b) => (output += b));
  const code = await new Promise((resolve) => {
    child.on("error", (e) => {
      output += String(e);
      resolve(-1);
    });
    child.on("close", (code) => resolve(code ?? -1));
  });
  fs.writeFileSync(path.join(logDir, `${id}.txt`), output);
  if (id === "headless-demo" && code === 0)
    fs.writeFileSync(path.join(root, "verification/demo.json"), stdout);
  records.push({
    id,
    executable: path.isAbsolute(executable)
      ? path.relative(root, executable)
      : executable,
    args,
    started_at: new Date(start).toISOString(),
    duration_ms: Date.now() - start,
    exit_code: code,
    result: code === 0 ? "pass" : "fail",
    output: `verification/logs/${id}.txt`,
  });
  fs.writeFileSync(
    path.join(root, "verification/progress.json"),
    JSON.stringify(
      {
        source_digest: sourceDigest,
        last_completed_at: new Date().toISOString(),
        records,
      },
      null,
      2,
    ) + "\n",
  );
  console.log(`[${id}] exit ${code} (${Date.now() - start} ms)`);
}
for (const command of commands) await run(...command);
const execVersion = (cmd, args) =>
  spawnSync(cmd, args, {
    cwd: root,
    encoding: "utf8",
    windowsHide: true,
  }).stdout?.trim();
const result = {
  format: "agentique-verification-run/0.1",
  completed_at: new Date().toISOString(),
  source_digest: sourceDigest,
  source_hashes: sourceHashes,
  git_commit:
    execVersion("git", [
      "-c",
      `safe.directory=${root.replaceAll("\\", "/")}`,
      "rev-parse",
      "HEAD",
    ]) || null,
  environment: {
    os: os.type(),
    release: os.release(),
    architecture: os.arch(),
    logical_cpus: os.cpus().length,
    physical_memory_bytes: os.totalmem(),
    node: process.version,
    rustc: execVersion("rustc", ["--version"]),
  },
  result: records.every((r) => r.exit_code === 0) ? "pass" : "incomplete",
  automated_checks_excluding_documented_validator_disagreement: records
    .filter((r) => r.id !== "independent-fixtures")
    .every((r) => r.exit_code === 0)
    ? "pass"
    : "fail",
  records,
};
fs.writeFileSync(
  path.join(root, "verification/results.json"),
  JSON.stringify(result, null, 2) + "\n",
);
await run("update-registers", "node", ["tools/registers.mjs"]);
fs.writeFileSync(
  path.join(root, "verification/results.json"),
  JSON.stringify(result, null, 2) + "\n",
);
console.log(
  `Evidence: verification/results.json. Result: ${result.result}. See documented standards and resource-control gaps before any release claim.`,
);
if (result.result !== "pass") process.exitCode = 1;
