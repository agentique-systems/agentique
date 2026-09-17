import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { once } from "node:events";
import assert from "node:assert/strict";
import { root } from "./extract.mjs";
const workspace = path.join(
  root,
  ".workspaces",
  `restart-${crypto.randomUUID()}.db`,
);
const port = 7354,
  base = `http://127.0.0.1:${port}`,
  token = "process-test-local-session-only";
const executable = path.join(
  root,
  "target",
  "debug",
  process.platform === "win32" ? "agq-server.exe" : "agq-server",
);
let child;
async function start() {
  child = spawn(
    executable,
    ["--workspace", workspace, "--port", String(port)],
    {
      cwd: root,
      windowsHide: true,
      env: {
        ...process.env,
        AGENTIQUE_SESSION_TOKEN: token,
        AGENTIQUE_ASSISTANT: "live",
        AGENTIQUE_AI_KEY: "",
        AGENTIQUE_AI_URL: "",
      },
      stdio: ["ignore", "ignore", "pipe"],
    },
  );
  let errors = "";
  child.stderr.on("data", (b) => (errors += b));
  for (let i = 0; i < 150; i++) {
    if (child.exitCode !== null) throw Error(errors);
    try {
      return await api("/api/agentique/state");
    } catch {}
    await new Promise((r) => setTimeout(r, 100));
  }
  throw Error("Server startup timeout: " + errors);
}
async function api(route, body) {
  const r = await fetch(base + route, {
    method: body ? "POST" : "GET",
    headers: {
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
    },
    body: body ? JSON.stringify(body) : undefined,
  });
  const json = await r.json();
  if (!r.ok) throw Object.assign(Error(json.message), json);
  return json;
}
async function kill() {
  const stopped = once(child, "exit");
  child.kill("SIGKILL");
  await stopped;
  child = null;
}
try {
  let state = await start();
  const initial = state.head_revision_id;
  const command = (payload) =>
    api("/api/agentique/commands", {
      command_id: crypto.randomUUID(),
      project_id: state.project_id,
      base_revision_id: state.head_revision_id,
      payload,
    });
  const act = (action) => command({ op: "act", action });
  const scenario = JSON.parse(
    fs.readFileSync(path.join(root, "scenarios/accepted.json"), "utf8"),
  );
  const saved = await act({ op: "save_scenario", scenario });
  const prepared = await act({
    op: "prepare_run",
    scenario_revision_id: saved.id,
  });
  const run = prepared.prepared_run_id;
  const control = async (operation) => {
    const current = await api(`/api/agentique/runs/${run}`);
    return act({
      op: "control_run",
      run_id: run,
      expected_control_version: current.control_version,
      operation,
    });
  };
  await control("initialise");
  const acknowledged = await control("step");
  const before = await api(`/api/agentique/runs/${run}`);
  const draft = await command({
    op: "save_draft",
    file: "models/AgentiqueBehaviour.sysml",
    source: "package Invalid {",
  });
  assert(draft.diagnostics.length > 0);
  await assert.rejects(
    api("/api/agentique/assistant", {
      message: "inspect",
      context: {
        project_id: state.project_id,
        revision_id: initial,
        selection: null,
        run_id: run,
      },
    }),
    (e) => e.code === "provider_unavailable",
  );
  await kill();
  state = await start();
  const recovered = await api(`/api/agentique/runs/${run}`);
  assert.equal(state.head_revision_id, initial);
  assert.equal(recovered.status, "interrupted");
  assert.equal(recovered.active, before.active);
  assert.equal(recovered.next_input, before.next_input);
  assert.deepEqual(recovered.trace_page, before.trace_page);
  assert.equal(state.drafts[0].source, "package Invalid {");
  const reset = await control("reset_as_new_run");
  assert.notEqual(reset.run_id, run);
  const report = {
    measured_at: new Date().toISOString(),
    command: "node tools/process-test.mjs",
    result: "pass",
    workspace,
    model_revision_id: initial,
    run_id: run,
    acknowledged_checkpoint: acknowledged.checkpoint_id,
    recovered_status: recovered.status,
    trace_records: recovered.trace_count,
    assertions: [
      "forced process termination",
      "durable checkpoint and trace preserved",
      "unfinished run interrupted",
      "invalid draft recovered",
      "provider unavailable does not prevent manual operations",
      "reset creates new run",
    ],
  };
  fs.writeFileSync(
    path.join(root, "verification/process-recovery.json"),
    JSON.stringify(report, null, 2) + "\n",
  );
  console.log(JSON.stringify(report, null, 2));
} finally {
  if (child) await kill();
}
