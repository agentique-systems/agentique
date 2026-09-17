import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { root, hash } from "./extract.mjs";
const baseline = process.argv.includes("--baseline");
const output = `verification/pilot-${baseline ? "baseline-" : ""}validation.json`;
const lock = JSON.parse(
  fs.readFileSync(
    path.join(
      root,
      baseline ? "standards/baseline-lock.json" : "standards/lock.json",
    ),
    "utf8",
  ),
);
const library = path.join(
  root,
  `.cache/pilot-libraries-${hash(JSON.stringify(lock)).slice(0, 16)}`,
);
function copySources(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const file = path.join(dir, entry.name);
    if (entry.isDirectory()) copySources(file);
    else if (/\.(sysml|kerml)$/.test(file)) {
      const bytes = fs.readFileSync(file);
      const sub =
        path.extname(file) === ".sysml"
          ? "Systems Library"
          : "Kernel Libraries";
      const dest = path.join(library, sub, entry.name);
      fs.mkdirSync(path.dirname(dest), { recursive: true });
      if (fs.existsSync(dest) && hash(fs.readFileSync(dest)) !== hash(bytes))
        throw Error("Changed staged library: " + dest);
      fs.writeFileSync(dest, bytes);
    }
  }
}
copySources(path.join(root, lock.library_source_root ?? "standards/libraries"));
const java =
  process.env.AGENTIQUE_JAVA ??
  (process.platform === "win32"
    ? path.join(root, ".cache/jdk-21.0.12.1+1/bin/java.exe")
    : "java");
const jar = path.join(
  root,
  ".cache/pilot-0.59.0/share/jupyter/kernels/sysml/jupyter-sysml-kernel-0.59.0-all.jar",
);
const files = [
  ...fs
    .readdirSync(path.join(root, "models"))
    .filter((f) => f.endsWith(".sysml"))
    .map((f) => "models/" + f),
  ...fs
    .readdirSync(path.join(root, "tests/fixtures"))
    .filter((f) => /\.(sysml|kerml)$/.test(f))
    .map((f) => "tests/fixtures/" + f),
];
const args = [
  "-Xmx2g",
  "--class-path",
  jar,
  "tools/ValidatePilot.java",
  library,
  output,
  ...files,
];
const result = spawnSync(java, args, {
  cwd: root,
  encoding: "utf8",
  windowsHide: true,
  maxBuffer: 8 * 1024 * 1024,
  timeout: 180000,
});
fs.writeFileSync(
  path.join(
    root,
    `verification/logs/pilot-${baseline ? "baseline-" : ""}validation.txt`,
  ),
  (result.stdout ?? "") + (result.stderr ?? "") + String(result.error ?? ""),
);
if (fs.existsSync(path.join(root, output))) {
  const report = JSON.parse(fs.readFileSync(path.join(root, output), "utf8"));
  Object.assign(report, {
    executed_at: new Date().toISOString(),
    command: { java, args },
    exit_code: result.status,
    jar_sha256: hash(fs.readFileSync(jar)),
    library_lock_sha256: hash(JSON.stringify(lock)),
    sources: Object.fromEntries(
      files.map((f) => [f, hash(fs.readFileSync(path.join(root, f)))]),
    ),
  });
  fs.writeFileSync(
    path.join(root, output),
    JSON.stringify(report, null, 2) + "\n",
  );
  console.log(
    JSON.stringify({ errors: report.errors, issues: report.issues }, null, 2),
  );
} else console.log(result.stdout, result.stderr, result.error);
process.exitCode = result.status ?? 1;
