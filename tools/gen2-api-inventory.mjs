import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const digest = (bytes) =>
  crypto.createHash("sha256").update(bytes).digest("hex");
const methods = new Set([
  "get",
  "post",
  "put",
  "patch",
  "delete",
  "head",
  "options",
  "trace",
]);

/** Build the inventory from checked-in authority; no network or acquisition. */
export function inventory(base = root) {
  const read = (file) => fs.readFileSync(path.join(base, file));
  const openapi = JSON.parse(read("standards/artifacts/OpenAPI.json"));
  const schema = JSON.parse(read("standards/artifacts/Schema.json"));
  const mapping = JSON.parse(read("standards/gen2-api-mapping.json"));
  const seen = new Set();
  const operations = [];
  for (const [route, item] of Object.entries(openapi.paths)) {
    for (const [method, operation] of Object.entries(item)) {
      if (!methods.has(method)) continue;
      if (!operation.operationId || seen.has(operation.operationId)) {
        throw Error(`Missing or duplicate operationId at ${method} ${route}`);
      }
      seen.add(operation.operationId);
      const support = mapping.operations[operation.operationId];
      if (
        !support ||
        !["supported", "partially_supported", "unsupported"].includes(
          support.status,
        ) ||
        !support.reason
      ) {
        throw Error(
          `Missing explicit support decision: ${operation.operationId}`,
        );
      }
      const resolveParameter = (parameter) =>
        parameter.$ref
          ? openapi.components.parameters[parameter.$ref.split("/").at(-1)]
          : parameter;
      const responseSchemas = Object.fromEntries(
        Object.entries(operation.responses)
          .filter(([status]) => /^2\d\d$/.test(status))
          .map(([status, response]) => [
            status,
            response.content?.["application/json"]?.schema ?? null,
          ]),
      );
      operations.push({
        operation_id: operation.operationId,
        method: method.toUpperCase(),
        path: route,
        parameters: [
          ...(item.parameters ?? []),
          ...(operation.parameters ?? []),
        ].map(resolveParameter),
        request_schema:
          operation.requestBody?.content?.["application/json"]?.schema ?? null,
        response_schemas: responseSchemas,
        ...support,
      });
    }
  }
  for (const id of Object.keys(mapping.operations)) {
    if (!seen.has(id))
      throw Error(`Mapping contains unknown operationId: ${id}`);
  }
  const normalizeRefs = (value) =>
    JSON.stringify(value, (key, item) =>
      key === "$ref" ? item.split("/").at(-1) : item,
    );
  for (const [name, definition] of Object.entries(schema.$defs)) {
    if (
      normalizeRefs(openapi.components.schemas[name]) !==
      normalizeRefs(definition)
    ) {
      throw Error(`OpenAPI / Schema authority mismatch: ${name}`);
    }
  }
  const counts = Object.fromEntries(
    ["supported", "partially_supported", "unsupported"].map((status) => [
      status,
      operations.filter((operation) => operation.status === status).length,
    ]),
  );
  return {
    format: "agentique-generation-2-systems-modeling-api-coverage/1",
    generation: 2,
    prefix: "/api/gen2",
    conformance: "not_claimed",
    authority: [
      { path: "SysAPI.pdf", document_id: "formal/2026-03-04" },
      { path: "standards/artifacts/OpenAPI.json", document_id: "ptc/25-02-30" },
      { path: "standards/artifacts/Schema.json", document_id: "ptc/25-02-31" },
    ].map((artifact) => ({ ...artifact, sha256: digest(read(artifact.path)) })),
    counts: { total: operations.length, ...counts },
    operations,
  };
}

if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const file = path.join(root, "standards/gen2-api-coverage.json");
  const output = JSON.stringify(inventory(), null, 2) + "\n";
  if (process.argv.includes("--check")) {
    if (fs.readFileSync(file, "utf8") !== output)
      throw Error("Generation-2 API inventory is stale");
  } else {
    fs.writeFileSync(file, output);
  }
  console.log(
    `Generation-2 API inventory: ${JSON.parse(output).counts.total} pinned operations.`,
  );
}
