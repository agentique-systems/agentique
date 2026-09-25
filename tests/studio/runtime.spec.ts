import { test, expect, APIRequestContext, Page } from "@playwright/test";
import { spawn, ChildProcess } from "node:child_process";
import {
  createWriteStream,
  existsSync,
  mkdirSync,
  mkdtempSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createServer } from "node:net";
import { performance } from "node:perf_hooks";
import type {
  Candidate,
  Inspector,
  Session,
  ViewProjection,
} from "../../console/src/studio/model";
import type { RuntimeStatus } from "../../console/src/studio/RuntimeSetup";

const evidence = resolve("verification/generated/agentique-studio-first-light");
const binary = resolve(
  process.env.AGENTIQUE_STUDIO_BINARY ??
    `target/debug/agq-studio${process.platform === "win32" ? ".exe" : ""}`,
);
const agentToken = "first-light-test-agent-read-propose";

async function freePort(): Promise<number> {
  const server = createServer();
  await new Promise<void>((done) => server.listen(0, "127.0.0.1", done));
  const port = (server.address() as { port: number }).port;
  await new Promise<void>((done, fail) =>
    server.close((error) => (error ? fail(error) : done())),
  );
  return port;
}
class StudioProcess {
  private child?: ChildProcess;
  readonly directory = mkdtempSync(join(tmpdir(), "agentique-first-light-"));
  readonly timings: Record<string, unknown> = {};
  url = "";
  async start(label: string) {
    if (!existsSync(binary))
      throw new Error(
        `Build the Studio host first: cargo build --locked --offline -p agq-studio (${binary})`,
      );
    mkdirSync(evidence, { recursive: true });
    const port = await freePort();
    this.url = `http://127.0.0.1:${port}`;
    const env = { ...process.env, AGENTIQUE_STUDIO_AGENT_TOKEN: agentToken };
    for (const key of [
      "AGENTIQUE_KERML_CACHE",
      "AGENTIQUE_SYSTEMS_CACHE",
      "AGENTIQUE_RUNTIME_DIR",
      "AGENTIQUE_STUDIO_OPERATOR_TOKEN",
    ])
      delete env[key];
    const started = performance.now();
    this.child = spawn(
      binary,
      [
        "--port",
        String(port),
        "--runtime-dir",
        join(this.directory, "runtime"),
        "--database",
        join(this.directory, "studio.sqlite"),
        "--root",
        process.cwd(),
      ],
      { env, windowsHide: true, stdio: ["ignore", "pipe", "pipe"] },
    );
    const log = createWriteStream(join(evidence, `${label}-host.log`));
    this.child.stdout!.pipe(log, { end: false });
    this.child.stderr!.pipe(log, { end: false });
    this.child.once("close", () => log.end());
    await expect
      .poll(
        async () => {
          if (this.child?.exitCode != null)
            throw new Error(
              `Studio exited with ${this.child.exitCode}; see ${label}-host.log`,
            );
          try {
            return (await fetch(`${this.url}/api/gen2/studio/runtime`)).status;
          } catch {
            return 0;
          }
        },
        { timeout: 30_000 },
      )
      .toBe(200);
    this.timings[`${label}_first_http_ms`] = performance.now() - started;
  }
  async stop() {
    const child = this.child;
    if (child && child.exitCode === null) {
      const ended = new Promise<void>((done) =>
        child.once("close", () => done()),
      );
      child.kill();
      await ended;
    }
    this.child = undefined;
  }
  retain(label: string) {
    writeFileSync(
      join(evidence, `${label}-measurements.json`),
      JSON.stringify(
        {
          database: join(this.directory, "studio.sqlite"),
          runtime: join(this.directory, "runtime"),
          timings: this.timings,
        },
        (key, value) => (key === "session_token" ? undefined : value),
        2,
      ),
    );
  }
}
async function api<T>(
  request: APIRequestContext,
  host: StudioProcess,
  path: string,
  token?: string,
  data?: unknown,
): Promise<T> {
  const result = await request.fetch(`${host.url}/api/gen2/studio${path}`, {
    method: data === undefined ? "GET" : "POST",
    data,
    headers: token ? { Authorization: `Bearer ${token}` } : {},
    timeout: 1_800_000,
  });
  expect(result.ok(), `${path}: ${await result.text()}`).toBeTruthy();
  return result.json();
}
async function world(page: Page, name: string) {
  await page
    .getByRole("navigation", { name: "Studio worlds" })
    .getByRole("button", { name, exact: false })
    .click();
}

test("fresh Studio serves genuine setup and fails closed without runtime assets", async ({
  page,
  request,
}) => {
  const host = new StudioProcess();
  try {
    await host.start("missing-runtime");
    await page.goto(`${host.url}/studio`);
    await expect(
      page.getByRole("heading", { name: "Your semantic runtime starts here." }),
    ).toBeVisible();
    await expect(
      page.getByText("KerML Operational v9", { exact: true }),
    ).toBeVisible();
    await expect(
      page.getByText("SysML Operational v3", { exact: true }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Use local bundle" }),
    ).toBeDisabled();
    await expect(page.getByTestId("semantic-node")).toHaveCount(0);
    const status = await api<RuntimeStatus>(request, host, "/runtime");
    expect(status.ready).toBe(false);
    expect(status.phase).toBe("setup_required");
    expect(
      (await request.get(`${host.url}/api/gen2/studio/session`)).status(),
    ).toBe(503);
    expect(existsSync(join(host.directory, "studio.sqlite"))).toBe(false);
    const agentStatus = await api<RuntimeStatus>(
      request,
      host,
      "/runtime",
      agentToken,
    );
    expect(agentStatus.session_token).toBeNull();
    expect(
      (
        await request.post(`${host.url}/api/gen2/studio/runtime/install`, {
          headers: { Authorization: `Bearer ${agentToken}` },
          data: { bundle: host.directory },
        })
      ).status(),
    ).toBe(403);
    await page
      .getByLabel("Local bundle path")
      .fill(join(host.directory, "missing.agq-runtime"));
    await page.getByRole("button", { name: "Use local bundle" }).click();
    await expect(
      page.getByRole("heading", { name: "Your semantic runtime starts here." }),
    ).toBeVisible();
    await expect(page.getByTestId("semantic-node")).toHaveCount(0);
    await page.screenshot({ path: join(evidence, "studio-runtime-setup.png") });
    host.timings.runtime = status;
  } finally {
    await host.stop();
    host.retain("missing-runtime");
  }
});

test("real accepted-runtime self-model operator sequence survives restart", async ({
  page,
  request,
}) => {
  test.skip(
    !process.env.AGENTIQUE_STUDIO_BUNDLE,
    "Requires an existing authenticated accepted bundle; no fixture or publication rebuild substitutes.",
  );
  const host = new StudioProcess();
  const started = performance.now();
  let viewRequests = 0;
  page.on("request", (request) => {
    if (request.url().endsWith("/view")) viewRequests++;
  });
  try {
    await host.start("cold");
    await page.goto(`${host.url}/studio`);
    await expect(
      page.getByRole("heading", { name: "Your semantic runtime starts here." }),
    ).toBeVisible();
    await page
      .getByLabel("Local bundle path")
      .fill(resolve(process.env.AGENTIQUE_STUDIO_BUNDLE!));
    await page.getByRole("button", { name: "Use local bundle" }).click();
    await expect(
      page.getByRole("heading", { name: "System World", exact: true }),
    ).toBeVisible({ timeout: 1_200_000 });
    await expect(
      page.getByRole("button", {
        name: "ModelingPlatform, PartDefinition",
        exact: true,
      }),
    ).toBeVisible();
    host.timings.cold_first_rendered_system_ms = performance.now() - started;
    host.timings.cold_startup = await api<RuntimeStatus>(
      request,
      host,
      "/runtime",
    );
    let session = await api<Session>(request, host, "/session");
    const project = session.project.id,
      revision = session.default_revision;
    expect(
      JSON.stringify(
        session.revisions.find((item) => item.revision_id === revision)
          ?.validation,
      ),
    ).toContain("Validated");
    const baseline = session.branches.find(
      (item) => item.name === "architecture-baseline",
    )!;
    expect(baseline).toBeTruthy();
    const visualStarted = performance.now(),
      beforeZoom = viewRequests;
    await page.getByRole("button", { name: "Zoom in", exact: true }).click();
    await page.getByRole("button", { name: "Zoom out", exact: true }).click();
    expect(viewRequests).toBe(beforeZoom);
    await page
      .getByRole("button", {
        name: "ModelingPlatform, PartDefinition",
        exact: true,
      })
      .click();
    await expect(
      page
        .locator(".studio-inspector")
        .getByRole("heading", { name: "ModelingPlatform", exact: true }),
    ).toBeVisible();
    host.timings.zoom_and_select_ms = performance.now() - visualStarted;
    await page.screenshot({
      path: join(evidence, "agentique-modeling-agentique.png"),
    });
    await world(page, "Graph");
    await expect(
      page.getByRole("heading", { name: "Graph World" }),
    ).toBeVisible();
    const beforeFilters = viewRequests;
    for (const family of [
      "Ownership",
      "Typing",
      "Specialization",
      "Subsetting",
      "Redefinition",
      "Connection",
      "Reference",
    ]) {
      await page.getByRole("checkbox", { name: family, exact: true }).uncheck();
      await page.getByRole("checkbox", { name: family, exact: true }).check();
    }
    expect(viewRequests).toBe(beforeFilters);
    await page
      .getByRole("checkbox", { name: "Standards", exact: true })
      .check();
    const derived = page.locator(".studio-edge.derived").first();
    await expect(derived).toBeVisible();
    await derived.focus();
    await derived.press("Enter");
    await page
      .getByRole("button", { name: "Why this relationship?", exact: false })
      .click();
    await expect(
      page.getByRole("dialog", { name: "Semantic explanation" }),
    ).toBeVisible();
    await expect(
      page.locator(".studio-evidence-graph article").first(),
    ).toBeVisible();
    await page.getByRole("button", { name: "Close explanation" }).click();
    await page
      .getByRole("combobox", { name: "Branch", exact: true })
      .selectOption(baseline.id);
    await expect(
      page.getByRole("combobox", { name: "Revision", exact: true }),
    ).toHaveValue(baseline.head);
    await page
      .getByRole("button", {
        name: "ModelingPlatform, PartDefinition",
        exact: true,
      })
      .click();
    await expect(
      page
        .locator(".studio-inspector")
        .getByRole("heading", { name: "ModelingPlatform", exact: true }),
    ).toBeVisible();
    await page
      .getByRole("combobox", { name: "Branch", exact: true })
      .selectOption(session.project.default_branch);
    await page.getByRole("button", { name: "Compare parent" }).click();
    await expect(page.locator(".studio-node.added").first()).toBeVisible();
    await page.getByRole("button", { name: "Close comparison" }).click();
    await page
      .getByRole("button", {
        name: "ModelingPlatform, PartDefinition",
        exact: true,
      })
      .click();
    await world(page, "Agents");
    await page.getByRole("button", { name: /Explore dependencies/ }).click();
    await expect(
      page.getByRole("heading", { name: "Graph World" }),
    ).toBeVisible();
    const preparation = performance.now();
    await page
      .getByRole("button", { name: "Create nested part", exact: false })
      .click();
    await page
      .getByRole("textbox", { name: "New part name" })
      .fill("firstLightObserver");
    const candidateResponse = page.waitForResponse(
      (response) =>
        response.url().endsWith("/candidates") &&
        response.request().method() === "POST",
      { timeout: 600_000 },
    );
    await page.getByRole("button", { name: "Prepare candidate" }).click();
    const candidate = (await (await candidateResponse).json()) as Candidate;
    host.timings.candidate_preparation_ms = performance.now() - preparation;
    const dialog = page.getByRole("dialog", {
      name: "Review candidate revision",
    });
    await expect(dialog).toBeVisible();
    await expect(dialog.locator(".studio-candidate-source")).toContainText(
      "firstLightObserver",
    );
    expect(candidate.validation).toBe("Working");
    expect(candidate.changes.declared?.added.length).toBeGreaterThan(0);
    expect(
      (await api<Session>(request, host, "/session")).default_revision,
    ).toBe(revision);
    const validation = performance.now();
    await dialog.getByRole("button", { name: "Validate", exact: true }).click();
    await expect(
      dialog.getByRole("button", { name: "Accept & commit" }),
    ).toBeEnabled({ timeout: 600_000 });
    host.timings.validation_ms = performance.now() - validation;
    const committed = performance.now();
    await dialog.getByRole("button", { name: "Accept & commit" }).click();
    await expect(dialog).not.toBeVisible({ timeout: 120_000 });
    session = await api<Session>(request, host, "/session");
    expect(session.default_revision).toBe(candidate.revision_id);
    host.timings.commit_ms = performance.now() - committed;
    await host.stop();
    const warm = performance.now();
    await host.start("warm");
    await page.goto(`${host.url}/studio`);
    await expect(
      page.getByRole("heading", { name: "System World", exact: true }),
    ).toBeVisible({ timeout: 600_000 });
    await expect(
      page.getByRole("combobox", { name: "Revision", exact: true }),
    ).toHaveValue(candidate.revision_id);
    host.timings.warm_first_rendered_system_ms = performance.now() - warm;
    host.timings.warm_startup = await api<RuntimeStatus>(
      request,
      host,
      "/runtime",
    );
    await world(page, "History");
    expect(
      await page.locator(".studio-revision-card").count(),
    ).toBeGreaterThanOrEqual(4);
    const restored = await api<Session>(request, host, "/session", agentToken);
    expect(restored.session_token).toBeNull();
    const definition = {
      version: 1,
      name: "Machine acceptance",
      kind: "SemanticGraph",
      focus: null,
      depth: 2,
      relationship_families: [
        "Ownership",
        "Typing",
        "Specialization",
        "Subsetting",
        "Redefinition",
        "Connection",
        "Reference",
      ],
      include_standard_library: true,
      hidden_elements: [],
    };
    const graph = await api<ViewProjection>(
      request,
      host,
      "/view",
      agentToken,
      { project, revision, definition },
    );
    expect(graph.revision_id).toBe(revision);
    const owner = graph.nodes.find(
      (node) =>
        node.name === "ModelingPlatform" &&
        node.semantic_kind === "PartDefinition",
    )!.id;
    expect(
      (
        await api<Inspector>(request, host, "/inspect", agentToken, {
          project,
          revision,
          element: owner,
        })
      ).revision_id,
    ).toBe(revision);
    const evidenceEdge = graph.edges.find(
      (edge) => edge.origin === "Derived" && edge.relationship_id,
    )!;
    await api(request, host, "/explain", agentToken, {
      project,
      revision,
      element: evidenceEdge.relationship_id,
    });
    await api(request, host, "/diff", agentToken, {
      project,
      from: revision,
      to: candidate.revision_id,
    });
    const machine = await api<Candidate>(
      request,
      host,
      "/candidates",
      agentToken,
      {
        project,
        branch: restored.project.default_branch,
        revision: restored.default_revision,
        command: { kind: "CreatePartUsage", owner, name: "machineObserver" },
      },
    );
    expect(machine.validation).toBe("Working");
    expect(
      (
        await request.post(
          `${host.url}/api/gen2/studio/candidates/${machine.id}/commit`,
          { headers: { Authorization: `Bearer ${agentToken}` } },
        )
      ).status(),
    ).toBe(403);
    expect(
      (await api<Session>(request, host, "/session", agentToken))
        .default_revision,
    ).toBe(candidate.revision_id);
    host.timings.operator_sequence =
      "passed against accepted runtime and durable repository";
  } finally {
    await host.stop();
    host.retain("real-model");
  }
});
