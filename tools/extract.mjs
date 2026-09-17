import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { fileURLToPath } from "node:url";

export const root = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
export const hash = (data) =>
  crypto.createHash("sha256").update(data).digest("hex");
export function safePath(base, name) {
  if (
    typeof name !== "string" ||
    !name ||
    name.includes("\\") ||
    name.includes(":") ||
    name.includes("\0") ||
    name.split("/").some((p) => !p || p === "." || p === "..")
  )
    throw Error(`Unsafe asset path: ${name}`);
  const target = path.resolve(base, name);
  if (!target.startsWith(base + path.sep))
    throw Error(`Outside repository: ${name}`);
  let cursor = base;
  for (const segment of name.split("/")) {
    cursor = path.join(cursor, segment);
    if (fs.existsSync(cursor) && fs.lstatSync(cursor).isSymbolicLink())
      throw Error(`Symlink refused: ${name}`);
  }
  return target;
}
export function assetsFrom(html) {
  // Read raw script text; neither eval nor a JavaScript runtime executes the document.
  const scripts = [
    ...html.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script\s*>/gi),
  ];
  const matches = scripts.filter(
    (m) =>
      /\bid\s*=\s*["']embedded-assets["']/i.test(m[1]) &&
      /\btype\s*=\s*["']application\/json["']/i.test(m[1]),
  );
  if (matches.length !== 1)
    throw Error("Expected one embedded-assets JSON script");
  const assets = JSON.parse(matches[0][2]);
  if (
    !assets ||
    Array.isArray(assets) ||
    typeof assets !== "object" ||
    Object.values(assets).some((v) => typeof v !== "string")
  )
    throw Error("Expected a path-to-text object");
  return assets;
}
export function extract(html, base, check = false) {
  const assets = assetsFrom(html);
  // Preflight the entire operation before writing anything. Existing differing files always fail.
  const entries = Object.entries(assets).map(([name, contents]) => {
    const target = safePath(base, name);
    if (fs.existsSync(target) && fs.readFileSync(target, "utf8") !== contents)
      throw Error(
        `Refusing to overwrite modified file: ${name}. Compare with the embedded original explicitly.`,
      );
    if (check && !fs.existsSync(target))
      throw Error(`Missing extracted file: ${name}`);
    return { name, contents, target };
  });
  if (!check)
    for (const { target, contents } of entries) {
      fs.mkdirSync(path.dirname(target), { recursive: true });
      if (!fs.existsSync(target))
        fs.writeFileSync(target, contents, { flag: "wx" });
    }
  return entries.map(({ name, contents }) => ({
    path: name,
    sha256: hash(contents),
  }));
}
if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const html = fs.readFileSync(
    path.join(root, "Agentique-Specification-v0.1.html"),
    "utf8",
  );
  const manifest = extract(html, root, process.argv.includes("--check"));
  fs.mkdirSync(path.join(root, "verification"), { recursive: true });
  const output =
    JSON.stringify({ source_sha256: hash(html), assets: manifest }, null, 2) +
    "\n";
  const target = path.join(root, "verification/extraction-lock.json");
  if (fs.existsSync(target) && fs.readFileSync(target, "utf8") !== output)
    throw Error("Extraction manifest differs");
  if (!fs.existsSync(target)) fs.writeFileSync(target, output, { flag: "wx" });
  console.log(`Verified ${manifest.length} extracted assets.`);
}
