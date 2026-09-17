// Explicit acquisition only; never imported by builds or tests.
import fs from "node:fs";
import { root, safePath } from "./extract.mjs";
import {
  readSysmlLock,
  librarySetPath,
  acquirePinned,
} from "./sysml-artifacts.mjs";

const lock = readSysmlLock(root);
const libraries = JSON.parse(fs.readFileSync(safePath(root, librarySetPath)));
const artifacts = [
  ...new Map(
    [...lock.artifacts, ...libraries.artifacts].map((a) => [a.path, a]),
  ).values(),
];
await acquirePinned(root, artifacts, async (source) => {
  const response = await fetch(source);
  if (!response.ok) throw new Error(`${source}: HTTP ${response.status}`);
  return Buffer.from(await response.arrayBuffer());
});
console.log(
  "SysML 2.0 inputs and exact library dependency set verified/reproduced without replacement.",
);
