import { test, expect } from "@playwright/test";
import fs from "node:fs";
const token = "agentique-e2e-local-session-only";
const headers = { Authorization: `Bearer ${token}` };
test("Cancellable model work and paged history use durable Engine results", async ({
  page,
  request,
}) => {
  const state = await (
    await request.get("/api/agentique/state", { headers })
  ).json();
  const large = {
    command_id: crypto.randomUUID(),
    project_id: state.project_id,
    base_revision_id: state.head_revision_id,
    payload: {
      op: "propose_change",
      edits: [
        {
          kind: "add_source",
          file: "large.sysml",
          source: `package Large { ${Array.from({ length: 30000 }, (_, i) => `part def P${i};`).join(" ")} }`,
        },
      ],
    },
  };
  const queued = await (
    await request.post("/api/agentique/jobs", { headers, data: large })
  ).json();
  expect(queued.acknowledged_commit).toBe(false);
  const cancel = await (
    await request.post(`/api/agentique/jobs/${queued.job_id}/cancel`, {
      headers,
    })
  ).json();
  expect(cancel.cancellation_requested).toBe(true);
  await expect
    .poll(
      async () =>
        (
          await (
            await request.get(`/api/agentique/jobs/${queued.job_id}`, {
              headers,
            })
          ).json()
        ).status,
    )
    .toBe("cancelled");
  const unchanged = await (
    await request.get("/api/agentique/state", { headers })
  ).json();
  expect(unchanged.head_revision_id).toBe(state.head_revision_id);
  expect(
    (
      await request.post("/api/agentique/jobs", {
        headers,
        data: {
          ...large,
          payload: { op: "save_draft", file: "x.sysml", source: "package X;" },
        },
      })
    ).status(),
  ).toBe(409);
  const send = async (payload: any, base = state.head_revision_id) => {
    const response = await request.post("/api/agentique/commands", {
      headers,
      data: {
        command_id: crypto.randomUUID(),
        project_id: state.project_id,
        base_revision_id: base,
        payload,
      },
    });
    expect(response.ok(), await response.text()).toBe(true);
    return response.json();
  };
  const proposed = await send({
    op: "propose_change",
    edits: [
      {
        kind: "add_source",
        file: "Controller.sysml",
        source: fs.readFileSync("tests/fixtures/Controller.sysml", "utf8"),
      },
    ],
  });
  expect(proposed.valid).toBe(true);
  await send({
    op: "act",
    action: { op: "commit_change", proposal_id: proposed.proposal_id },
    approval_id: null,
  });
  await page.goto("/expert#token=" + token);
  await page
    .getByText("Edit scenario inputs, bindings and limits", { exact: true })
    .click();
  await page.locator("#scenario").fill(
    JSON.stringify({
      execution_contract: "AGQ-SEQ-01",
      selected_behaviour_path: "Controller::Mode",
      bindings: {
        enabled: { kind: "boolean", value: true },
        threshold: { kind: "integer", value: "10" },
      },
      inputs: Array.from({ length: 511 }, (_, i) => ({
        ordinal: i + 1,
        receiver_path: "Controller::Mode.commands",
        payload_type: "Controller::Switch",
        values: {},
      })),
      stop_when_active_state: null,
      limits: {
        max_input_deliveries: 600,
        max_semantic_steps: 600,
        max_trace_records: 2000,
        max_memory_bytes: 67108864,
      },
    }),
  );
  await page
    .getByRole("button", { name: "Prepare new run", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Initialise", exact: true }),
  ).toBeEnabled();
  await page.getByRole("button", { name: "Initialise", exact: true }).click();
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(page.locator(".run-stats")).toContainText("input_exhausted", {
    timeout: 45000,
  });
  await page
    .getByRole("button", { name: "Next trace page", exact: true })
    .click();
  await expect(
    page.getByRole("navigation", { name: "Trace pages" }),
  ).toContainText("1001–1023 of 1023");
  await expect(page.locator(".experiment tbody tr")).toHaveCount(23);
  await page
    .getByRole("button", { name: "Previous trace page", exact: true })
    .click();
  await expect(page.locator(".experiment tbody tr")).toHaveCount(1000);
});
test("Console lifecycle, source navigation, shared context and reviewed Assistant change", async ({
  page,
  request,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto("/expert#token=" + token);
  await expect(
    page.getByRole("heading", { name: "A model we can run." }),
  ).toBeVisible();
  await expect(page.getByText("TEST MODE", { exact: true })).toBeVisible();
  const initial = await (
    await request.get("/api/agentique/state", { headers })
  ).json();
  await page
    .getByRole("button", { name: "Rearrange views", exact: true })
    .click();
  const afterLayout = await (
    await request.get("/api/agentique/state", { headers })
  ).json();
  expect(initial.model.source_digest).toBe(afterLayout.model.source_digest);
  await page
    .getByRole("button", { name: "Rearrange views", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Prepare new run", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Initialise", exact: true }),
  ).toBeEnabled();
  await page.getByRole("button", { name: "Initialise", exact: true }).click();
  await expect(page.locator(".run-stats")).toContainText("idle");
  await page.getByRole("button", { name: "Step", exact: true }).click();
  await expect(page.locator(".run-stats")).toContainText("checking");
  await page.getByRole("button", { name: "Step", exact: true }).click();
  await expect(page.locator(".run-stats")).toContainText("accepted");
  await expect(page.locator(".run-stats")).toContainText("condition_met");
  await page
    .locator(".experiment")
    .getByRole("button", { name: "acceptCheck", exact: true })
    .click();
  await expect(page.locator(".properties h2")).toHaveText("acceptCheck");
  await expect(
    page.getByRole("heading", { name: "RequestLifecycle", exact: true }),
  ).toBeVisible();
  await page.getByLabel("Ask about this model").fill("inspect");
  await page.getByRole("button", { name: "Send", exact: false }).click();
  await expect(page.locator(".message.assistant").last()).toContainText(
    "acceptCheck",
  );
  await page.getByLabel("Ask about this model").fill("rename acceptValidated");
  await page.getByRole("button", { name: "Send", exact: false }).click();
  await expect(
    page.getByRole("button", { name: "Approve and execute", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Approve and execute", exact: true })
    .click();
  await expect(page.locator(".properties h2")).toHaveText("acceptValidated");
  await expect(page.locator(".pin")).toContainText("Older than accepted model");
  await page.locator(".topbar").scrollIntoViewIfNeeded();
  await page.screenshot({
    path: "verification/generated/screenshots/console-desktop.png",
    fullPage: true,
  });
  const updated = await (
    await request.get("/api/agentique/state", { headers })
  ).json();
  expect(updated.head_revision_id).not.toBe(initial.head_revision_id);
  const before = initial.model.elements.find(
    (e: any) => e.name === "acceptCheck",
  );
  expect(
    updated.model.elements.find((e: any) => e.name === "acceptValidated").id,
  ).toBe(before.id);
  await page.getByRole("button", { name: "rejected", exact: true }).click();
  await page
    .getByRole("button", { name: "Prepare new run", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Initialise", exact: true }),
  ).toBeEnabled();
  await page.getByRole("button", { name: "Initialise", exact: true }).click();
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(page.locator(".run-stats")).toContainText("rejected");
  await page.getByRole("button", { name: "exhausted", exact: true }).click();
  await page
    .getByRole("button", { name: "Prepare new run", exact: true })
    .click();
  await page.getByRole("button", { name: "Initialise", exact: true }).click();
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(page.locator(".run-stats")).toContainText("input_exhausted");
  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({
    path: "verification/generated/screenshots/console-mobile.png",
    fullPage: true,
  });
  expect(errors).toEqual([]);
});
test("API authentication, scope, identity mapping and revision checks", async ({
  request,
}) => {
  expect((await request.get("/api/agentique/state")).status()).toBe(401);
  const state = await (
    await request.get("/api/agentique/state", { headers })
  ).json();
  const projects = await (
    await request.get("/api/model/projects", { headers })
  ).json();
  expect(projects[0]["@id"]).toBe(state.project_id);
  const schema = JSON.parse(
    fs.readFileSync("standards/artifacts/Schema.json", "utf8"),
  );
  const branchId = projects[0].defaultBranch["@id"];
  const branch = await (
    await request.get(
      `/api/model/projects/${state.project_id}/branches/${branchId}`,
      { headers },
    )
  ).json();
  const commit = await (
    await request.get(
      `/api/model/projects/${state.project_id}/commits/${state.head_revision_id}`,
      { headers },
    )
  ).json();
  for (const [name, data] of [
    ["Project", projects[0]],
    ["Branch", branch],
    ["Commit", commit],
  ] as const) {
    expect(Object.keys(data).sort()).toEqual(
      Object.keys(schema.$defs[name].properties).sort(),
    );
    for (const required of schema.$defs[name].required)
      expect(data).toHaveProperty(required);
  }
  const first = await request.get(
    `/api/model/projects/${state.project_id}/commits/${state.head_revision_id}/elements?excludeUsed=true&page[size]=2`,
    { headers },
  );
  const firstData = await first.json();
  expect(firstData).toHaveLength(2);
  const next = first.headers().link.match(/<([^>]+)>; rel="next"/)?.[1];
  expect(next).toBeTruthy();
  const second = await (await request.get(next!, { headers })).json();
  expect(second[0]["@id"]).not.toBe(firstData[0]["@id"]);
  const elements = await (
    await request.get(
      `/api/model/projects/${state.project_id}/commits/${state.head_revision_id}/elements`,
      { headers },
    )
  ).json();
  expect(elements[0]["@id"]).toBeTruthy();
  expect(
    (await request.get("/api/model/projects/wrong", { headers })).status(),
  ).toBe(404);
  const stale = await request.post("/api/agentique/commands", {
    headers,
    data: {
      command_id: crypto.randomUUID(),
      project_id: state.project_id,
      base_revision_id: "stale",
      payload: { op: "save_draft", file: "a.sysml", source: "package A;" },
    },
  });
  expect(stale.status()).toBe(409);
  const forged = await request.post("/api/agentique/commands", {
    headers,
    data: {
      command_id: crypto.randomUUID(),
      project_id: state.project_id,
      base_revision_id: state.head_revision_id,
      actor: "operator",
      payload: { op: "save_draft", file: "a.sysml", source: "package A;" },
    },
  });
  expect(forged.ok()).toBe(false);
  const events = await (
    await request.get("/api/agentique/events?after=0&limit=1000", { headers })
  ).json();
  expect(
    events.events.every(
      (e: any, i: number) => !i || e.sequence > events.events[i - 1].sequence,
    ),
  ).toBe(true);
});
test("Keyboard inspection and recoverable source drafts", async ({
  page,
  request,
}) => {
  await page.goto("/expert#token=" + token);
  await expect(
    page.getByRole("heading", { name: "A model we can run." }),
  ).toBeVisible();
  const original = await (
    await request.get("/api/agentique/state", { headers })
  ).json();
  await page.getByLabel("Filter model elements").focus();
  await page.keyboard.type("Engine");
  await page.keyboard.press("Tab");
  await page.keyboard.press("Enter");
  await expect(page.locator(".properties h2")).toContainText("Engine");
  await page.getByText("Source draft editor", { exact: true }).click();
  await page.locator("#source-draft").fill("package Incomplete {");
  await page
    .getByRole("button", { name: "Save and validate draft", exact: true })
    .click();
  await expect(page.locator(".message.engine").last()).toContainText(
    "Draft saved",
  );
  const saved = await (
    await request.get("/api/agentique/state", { headers })
  ).json();
  expect(saved.model.source_digest).toBe(original.model.source_digest);
  expect(
    saved.drafts[0].diagnostics.some((d: any) => d.severity === "error"),
  ).toBe(true);
  await page.reload();
  await expect(page.getByLabel("Filter model elements")).toBeVisible();
  await page.getByLabel("Filter model elements").focus();
  await page.keyboard.type("Engine");
  await page.keyboard.press("Tab");
  await page.keyboard.press("Enter");
  await page.getByText("Source draft editor", { exact: true }).click();
  await expect(page.locator("#source-draft")).toHaveValue(
    "package Incomplete {",
  );
  await page.getByLabel("Ask about this model").focus();
  await page.keyboard.type("inspect");
  await page.keyboard.press("Control+Enter");
  await expect(page.locator(".message.assistant").last()).toContainText(
    "Engine",
  );
  await page
    .getByRole("button", { name: "Reset as new run", exact: true })
    .isVisible();
});
