import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";
import { inventory } from "./gen2-api-inventory.mjs";

test("Gen2 operation coverage exactly tracks all pinned operation IDs and both schema artifacts", () => {
  const result = inventory();
  assert.equal(result.counts.total, 35);
  assert.deepEqual(
    result,
    JSON.parse(
      fs.readFileSync(
        new URL("../standards/gen2-api-coverage.json", import.meta.url),
      ),
    ),
  );
  assert.equal(
    result.operations.find((operation) => operation.operation_id === "merge")
      .status,
    "unsupported",
  );
  assert.equal(
    result.operations.find(
      (operation) => operation.operation_id === "postCommitByProject",
    ).status,
    "unsupported",
  );
});
