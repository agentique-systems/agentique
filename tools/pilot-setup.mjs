import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { unzipSync } from "fflate";
import { decompress } from "fzstd";
import { root, hash } from "./extract.mjs";
const artifacts = [
  {
    file: "jupyter-sysml-kernel-0.59.0.conda",
    version: "SysML pilot 0.59.0 (2026-04)",
    url: "https://api.anaconda.org/download/conda-forge/jupyter-sysml-kernel/0.59.0/noarch/jupyter-sysml-kernel-0.59.0-pyhd8ed1ab_0.conda",
    sha256: "b7269d3e3e1a3b89dae0b6e8185d3c0face99504b7f0ec86be75ffe002f82779",
    provenance:
      "Official pilot 2026-04 install/jupyter/install.sh selects this conda-forge package.",
  },
  {
    file: "temurin-jdk21.zip",
    version: "Eclipse Temurin 21.0.12.1+1 Windows x64",
    url: "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.12.1%2B1/OpenJDK21U-jdk_x64_windows_hotspot_21.0.12.1_1.zip",
    sha256: "f9d6e191ab098c0d416e7d588a24420a8621cd2f4720dab2459b8b7b2d2d8b4e",
    provenance: "Adoptium v3 assets API, vendor checksum verified.",
  },
];
const cache = path.join(root, ".cache");
fs.mkdirSync(cache, { recursive: true });
for (const a of artifacts) {
  if (a.file === "temurin-jdk21.zip" && process.platform !== "win32") continue;
  const p = path.join(cache, a.file);
  if (!fs.existsSync(p)) {
    const r = await fetch(a.url);
    if (!r.ok) throw Error(`Download failed: ${r.status}`);
    fs.writeFileSync(p, Buffer.from(await r.arrayBuffer()), { flag: "wx" });
  }
  if (hash(fs.readFileSync(p)) !== a.sha256)
    throw Error(`Artifact hash mismatch: ${a.file}`);
}
function extract(archive, destination) {
  const list = spawnSync("tar", ["-tf", archive], { encoding: "utf8" });
  if (list.status !== 0) throw Error(list.stderr);
  for (const name of list.stdout.trim().split(/\r?\n/)) {
    const target = path.resolve(destination, name);
    if (
      name.includes("\\") ||
      name.split("/").includes("..") ||
      !target.startsWith(destination + path.sep)
    )
      throw Error(`Unsafe archive entry ${name}`);
  }
  fs.mkdirSync(destination, { recursive: true });
  const result = spawnSync("tar", ["-xf", archive, "-C", destination], {
    encoding: "utf8",
  });
  if (result.status !== 0) throw Error(result.stderr);
}
const dest = path.join(cache, "pilot-0.59.0");
if (!fs.existsSync(path.join(dest, ".acquired"))) {
  const entries = unzipSync(
    fs.readFileSync(path.join(cache, artifacts[0].file)),
  );
  const pkg = Object.entries(entries).find(
    ([name]) => name.startsWith("pkg-") && name.endsWith(".tar.zst"),
  );
  const tar = path.join(cache, "pilot-package.tar");
  fs.writeFileSync(tar, decompress(pkg[1]));
  extract(tar, dest);
  fs.writeFileSync(path.join(dest, ".acquired"), artifacts[0].sha256);
}
if (
  process.platform === "win32" &&
  !fs.existsSync(path.join(cache, "jdk-21.0.12.1+1/bin/java.exe"))
)
  extract(path.join(cache, artifacts[1].file), cache);
fs.writeFileSync(
  path.join(root, "verification/pilot-provenance.json"),
  JSON.stringify(
    {
      artifacts,
      acquired_at: new Date().toISOString(),
      note: "External validator tooling only; Engine continues to use original OMG library bytes.",
    },
    null,
    2,
  ) + "\n",
);
console.log(dest);
