import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { extract, assetsFrom, safePath } from "./extract.mjs";
const html = (a) =>
  `<script id="embedded-assets" type="application/json">${JSON.stringify(a)}</script><script>throw Error('must never execute')</script>`;
test("extract is reproducible, nonexecuting, and refuses modified output atomically", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "agq-extract-"));
  const s = html({ "a/model.sysml": "package A;" });
  extract(s, root);
  assert.equal(
    fs.readFileSync(path.join(root, "a/model.sysml"), "utf8"),
    "package A;",
  );
  extract(s, root, true);
  assert.throws(
    () =>
      extract(
        html({ "new.sysml": "package New;", "a/model.sysml": "changed" }),
        root,
      ),
    /Refusing/,
  );
  assert.equal(fs.existsSync(path.join(root, "new.sysml")), false);
});
test("unsafe paths and duplicate embedded scripts rejected", () => {
  for (const p of ["../a", "C:/a", "a\\b", "/tmp/a", "a/../b", "a//b"])
    assert.throws(() => safePath(path.resolve("."), p));
  assert.throws(() => assetsFrom(html({}) + html({})));
});
