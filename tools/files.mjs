// Shared helpers for the standards pinning tools.
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
