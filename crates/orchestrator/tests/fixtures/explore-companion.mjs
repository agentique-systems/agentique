// A scripted stand-in for the Claude Agent companion (protocol 2) playing
// the roles of an exploring cycle for the Orchestrator's tests (C-54): no
// SDK, no network, no key. The role comes from the system prompt; files in
// the repository steer it:
// - DELEGATE: the lead, planning an objective whose intent says
//   DELEGATE-ME, delegates a child (after one delegation over budget, which
//   is refused, when DELEGATE-TOO-MUCH is there too), and plans once the
//   child's result is back;
// - DELEGATE-MANY: the planning lead delegates in every turn until the
//   Orchestrator refuses, then plans;
// - WAIT-MESSAGE: the implementer waits for the Operator's message;
// - WEAKEN: the implementer's first attempt has the test but not the fix,
//   and its repair weakens the test until it passes anywhere.
// The lead plans the History panel, proposes to fix the first reproduced
// finding (f1) with a test that fails on the base, and says what the
// Operator wrote; the implementer writes the fix (FIXED, which the
// stand-in Studio reads) and its test; the reviewer approves.
import { createInterface } from "node:readline";
import { existsSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const out = (message) => process.stdout.write(JSON.stringify(message) + "\n");
const lines = createInterface({ input: process.stdin });
const queue = [];
let wake = null;
lines.on("line", (line) => {
  queue.push(JSON.parse(line));
  if (wake) {
    wake();
    wake = null;
  }
});
lines.on("close", () => {
  queue.push({ type: "eof" });
  if (wake) wake();
});
const next = async () => {
  while (queue.length === 0) {
    await new Promise((resolve) => (wake = resolve));
  }
  return queue.shift();
};
// Messages the runtime sends while the turn runs (the Operator's).
const said = [];

out({ type: "ready", protocol: 2, sdk: "test", claudeCode: "test", node: process.version });
const start = await next();
if (start.type !== "start") process.exit(5);
const o = start.options;
const planning = o.systemPrompt.includes("planning this cycle's exploration");
const role = (o.systemPrompt.match(/Your role: (?:independent )?(\w+)/) ?? [])[1] ?? "unknown";
const session = `session-${role}-${Date.now()}`;
out({
  type: "init",
  sessionId: session,
  model: "deepseek-v4-pro",
  claudeCode: "test",
  tools: ["Read", "mcp__agentique__read_model"],
  mcpServers: [{ name: "agentique", status: "connected" }],
  permissionMode: "default",
  apiKeySource: "ANTHROPIC_API_KEY",
  skills: [],
  agents: [],
  plugins: [],
});

let n = 0;
const call = async (name, input) => {
  n += 1;
  const id = `toolu_${n}`;
  out({ type: "assistant", id: `msg_${n}`, model: "deepseek-v4-pro", content: [
    { type: "tool_use", id, name: `mcp__agentique__${name}`, input },
  ] });
  out({ type: "tool_call", call: `c${n}`, toolUseId: id, name, input });
  for (;;) {
    const message = await next();
    if (message.type === "message") said.push(message.text);
    if (message.type === "tool_result") return message;
    if (message.type === "eof" || message.type === "interrupt") process.exit(0);
  }
};
const end = (text) =>
  out({ type: "assistant", id: `msg_end_${n}`, model: "deepseek-v4-pro", content: [{ type: "text", text }] });
const here = (file) => existsSync(join(o.cwd, file));

if (role === "lead" && planning) {
  const delegating = here("DELEGATE") && o.prompt.includes("DELEGATE-ME") && !o.prompt.includes("has ended");
  if (here("DELEGATE-MANY") && (o.prompt.includes("DELEGATE-ME") || o.prompt.includes("has ended"))) {
    // Delegates in every turn until the Orchestrator refuses, then plans.
    const answer = await call("delegate", { instruction: "Look at the History panel again", usd: 0.2, steps: 10 });
    if (answer.isError) {
      await call("submit_exploration", { goal: "Look at the History panel and its buttons" });
      end("Planned.");
    } else {
      end("Delegated.");
    }
  } else if (delegating) {
    if (here("DELEGATE-TOO-MUCH")) {
      const refused = await call("delegate", { instruction: "Explore everything", usd: 999, steps: 5 });
      if (!refused.isError) process.exit(7);
    }
    await call("delegate", {
      instruction: "Look closely at the History panel",
      focus: "History",
      usd: 0.3,
      steps: 30,
    });
    end("Delegated.");
  } else {
    // What the Operator wrote, given with the brief, goes into the goal.
    const wrote = (o.prompt.match(/The Operator wrote: (.*)/) ?? [])[1];
    await call("submit_exploration", {
      goal: `Look at the History panel and its buttons${wrote ? ` (${wrote})` : ""}`,
    });
    end("Planned.");
  }
} else if (role === "lead") {
  const finding = o.prompt.includes("f1:") ? "f1" : undefined;
  const wrote = (o.prompt.match(/The Operator wrote: (.*)/) ?? [])[1];
  const answer = await call("submit_proposal", {
    title: "Label the History panel's Archive button",
    kind: "usability",
    why: `The Archive button has no readable label.${wrote ? ` The Operator wrote: ${wrote}` : ""}`,
    parts: ["Demo"],
    plan: ["Label it", "Test it"],
    criteria: [
      { id: "c1", statement: "The fix is there", check: { kind: "command", program: ["node", "--test", "fixed.test.mjs"] } },
    ],
    ...(finding ? { finding } : {}),
  });
  end(`Proposed (${answer.isError ? `refused: ${answer.content}` : "accepted"}).`);
} else if (role === "implementer") {
  if (here("WAIT-MESSAGE")) {
    out({ type: "text", text: "Working." });
    const until = Date.now() + 30000;
    while (said.length === 0 && Date.now() < until) {
      if (queue.length === 0) {
        await new Promise((resolve) => setTimeout(resolve, 100));
        continue;
      }
      const message = queue.shift();
      if (message.type === "message") said.push(message.text);
      if (message.type === "eof" || message.type === "interrupt") process.exit(0);
    }
  }
  const repairing = o.prompt.includes("Repair round");
  if (here("WEAKEN") && repairing) {
    // The repair "fixes" the failing test by weakening it until it passes
    // anywhere: the evidence on the base must be made again, and fail.
    writeFileSync(join(o.cwd, "FIXED"), "labelled\n");
    writeFileSync(
      join(o.cwd, "fixed.test.mjs"),
      'import { test } from "node:test";\nimport assert from "node:assert";\ntest("the Archive button is labelled", () => {\n  assert.ok(true);\n});\n',
    );
  } else {
    if (!here("WEAKEN")) {
      writeFileSync(join(o.cwd, "FIXED"), said.length ? said.join("\n") + "\n" : "labelled\n");
    }
    writeFileSync(
      join(o.cwd, "fixed.test.mjs"),
      'import { test } from "node:test";\nimport assert from "node:assert";\nimport { existsSync } from "node:fs";\ntest("the Archive button is labelled", () => {\n  assert.ok(existsSync("FIXED"));\n});\n',
    );
  }
  await call("submit_implementation", { summary: `Labelled it.${said.length ? ` Heard: ${said.join("; ")}` : ""}` });
  end("Implemented.");
} else if (role === "reviewer") {
  await call("submit_review", { verdict: "approve", findings: [], test_changes_accepted: false });
  end("Reviewed.");
} else {
  end(`No script for ${role}.`);
}
out({
  type: "result",
  isError: false,
  subtype: "success",
  stopReason: "end_turn",
  numTurns: 2,
  costUsd: 0.01,
  usage: { inputTokens: 1000, outputTokens: 100, cacheReadTokens: 0, cacheWriteTokens: 0 },
  usageByModel: { [o.model ?? "deepseek-v4-pro"]: { inputTokens: 1000, outputTokens: 100, cacheReadTokens: 0, cacheWriteTokens: 0 } },
  sessionId: session,
  denials: [],
  errors: [],
});
setTimeout(() => process.exit(0), 50);
