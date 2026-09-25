/** Synthetic transport-contract tests. These do not establish real semantic or durable acceptance. */
import { test, expect, Page } from "@playwright/test";
import type {
  Candidate,
  Inspector,
  Session,
  ViewProjection,
} from "../../console/src/studio/model";

const R1 = "11111111-0000-0000-0000-000000000001";
const R2 = "22222222-0000-0000-0000-000000000002";
const R3 = "33333333-0000-0000-0000-000000000003";
const root = "fixture-root",
  child = "fixture-child",
  relationship = "fixture-ownership";
const token = "fixture-operator-capability";
function projection(revision: string, candidate = false): ViewProjection {
  return {
    revision_id: revision,
    view: {
      version: 1,
      name: "Contract fixture architecture",
      kind: "Architecture",
      focus: null,
      depth: 2,
      relationship_families: ["Ownership", "Typing", "Connection"],
      include_standard_library: false,
      hidden_elements: [],
    },
    nodes: [
      {
        id: root,
        revision_id: revision,
        semantic_kind: "PartDefinition",
        name: "FixtureSystem",
        qualified_name: "Fixture::FixtureSystem",
        owner: null,
        origin: "Authored",
        source_available: true,
        features: [
          { id: child, name: "controller", semantic_kind: "PartUsage" },
        ],
        counts: { parts: 1, ports: 0, requirements: 0 },
        badges: [],
      },
      {
        id: child,
        revision_id: revision,
        semantic_kind: "PartUsage",
        name: "controller",
        qualified_name: "Fixture::FixtureSystem::controller",
        owner: root,
        origin: "Authored",
        source_available: true,
        features: [],
        counts: { parts: 0, ports: 0, requirements: 0 },
        badges: [],
      },
      ...(candidate
        ? [
            {
              id: "fixture-added",
              revision_id: revision,
              semantic_kind: "PartUsage",
              name: "redundantController",
              qualified_name: "Fixture::FixtureSystem::redundantController",
              owner: root,
              origin: "Authored" as const,
              source_available: true,
              features: [],
              counts: { parts: 0, ports: 0, requirements: 0 },
              badges: [],
            },
          ]
        : []),
    ],
    edges: [
      {
        id: "ownership-edge",
        relationship_id: relationship,
        revision_id: revision,
        family: "Ownership",
        semantic_kind: "OwningMembership",
        source: root,
        target: child,
        origin: "Derived",
        rule_id: "fixture-rule",
        label: "owns",
        order: 0,
        directed: true,
      },
    ],
    groups: [],
    metadata: {},
  };
}
function session(committed: boolean): Session {
  return {
    session_token: token,
    project: {
      id: "fixture-project",
      name: "Contract fixture",
      default_branch: "fixture-main",
    },
    branches: [{ id: "fixture-main", name: "main", head: committed ? R3 : R2 }],
    revisions: [
      { revision_id: R1, parent_revision_id: null, validation: "Validated" },
      {
        revision_id: R2,
        parent_revision_id: R1,
        validation: { Validated: {} },
      },
      ...(committed
        ? [{ revision_id: R3, parent_revision_id: R2, validation: "Validated" }]
        : []),
    ],
    default_revision: committed ? R3 : R2,
    saved_views: [],
  };
}
async function installFixture(page: Page) {
  const requests: { path: string; body: any; method: string }[] = [];
  let committed = false,
    validated = false;
  const candidate = (): Candidate => ({
    id: "fixture-candidate",
    revision_id: R3,
    base_revision: R2,
    validation: validated ? "Validated" : "Working",
    projection: projection(R3, true),
    changes: {},
    source_preview: {
      path: "fixture.sysml",
      before: "part def FixtureSystem { part controller; }",
      after:
        "part def FixtureSystem { part controller; part redundantController; }",
    },
  });
  await page.route("**/api/gen2/studio/**", async (route) => {
    const request = route.request(),
      path = new URL(request.url()).pathname.replace("/api/gen2/studio", "");
    const body = request.postDataJSON();
    requests.push({ path, body, method: request.method() });
    if (path !== "/session")
      expect(request.headers().authorization).toBe(`Bearer ${token}`);
    let result: unknown;
    if (path === "/session") result = session(committed);
    else if (path === "/view")
      result = {
        ...projection(body.revision, body.revision === R3),
        view: body.definition,
      };
    else if (path === "/inspect") {
      const node = projection(body.revision, body.revision === R3).nodes.find(
        (item) => item.id === body.element,
      )!;
      result = {
        revision_id: body.revision,
        element: node,
        owner: node.owner
          ? { id: root, name: "FixtureSystem", semantic_kind: "PartDefinition" }
          : null,
        effective_types: [],
        owned_features: node.features,
        effective_features: node.features,
        specializations: [],
        subsettings: [],
        redefinitions: [],
        relationships: projection(body.revision).edges,
        source: {
          document_id: "fixture-document",
          path: "fixture.sysml",
          source_revision_id: "fixture-source",
          start: 0,
          end: 45,
        },
        queries: [
          {
            name: "Owned features",
            completeness: "Complete",
            diagnostics: [],
            positive_dependency_count: 1,
            search_dependency_count: 0,
          },
        ],
        profile: "Contract fixture only",
      } satisfies Inspector;
    } else if (path === "/explain")
      result = {
        revision_id: body.revision,
        subject_id: body.element,
        origin: "Derived",
        rule_id: "fixture-rule",
        rule_name: "Fixture derivation",
        profile: "Contract fixture only",
        nodes: [
          {
            id: "subject",
            element_id: child,
            label: "controller",
            kind: "Element",
          },
          {
            id: "rule",
            element_id: null,
            label: "Fixture derivation",
            kind: "Rule",
          },
        ],
        edges: [
          {
            source: "subject",
            target: "rule",
            label: "established by",
            presentation_only: true,
          },
        ],
        evidence_count: 1,
        truncated: false,
      };
    else if (path === "/source")
      result = {
        revision_id: body.revision,
        path: "fixture.sysml",
        source: "part def FixtureSystem { part controller; }",
        start: 0,
        end: 45,
      };
    else if (path === "/diff")
      result = {
        from: body.from,
        to: body.to,
        before: projection(body.from),
        after: projection(body.to, true),
        diff: {
          declared: { added: [], removed: [], changed: [root] },
          relationships_changed: [{ Element: relationship }],
        },
      };
    else if (path === "/agent")
      result = {
        message: "A dependency view of the selected fixture element.",
        view: {
          ...projection(body.revision),
          view: {
            ...projection(body.revision).view,
            name: "Agent dependency view",
            kind: "SemanticGraph",
          },
        },
        decision: { provider: "deterministic fixture" },
      };
    else if (path === "/candidates") result = candidate();
    else if (path.endsWith("/validate")) {
      validated = true;
      result = candidate();
    } else if (path.endsWith("/commit")) {
      expect(validated).toBe(true);
      committed = true;
      result = { revision_id: R3 };
    } else if (request.method() === "DELETE") result = { rejected: true };
    else result = { saved: true };
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(result),
    });
  });
  return requests;
}

test("Studio reports missing durable inputs without substituting model content", async ({
  page,
}) => {
  await page.route("**/api/gen2/studio/session", (route) =>
    route.fulfill({
      status: 503,
      contentType: "application/json",
      body: JSON.stringify({
        description: "Accepted publication caches are unavailable.",
      }),
    }),
  );
  await page.goto("/studio");
  await expect(page.getByText("Repository connection needed")).toBeVisible();
  await page.getByText("Setup details", { exact: true }).click();
  await expect(
    page.getByText("Accepted publication caches are unavailable."),
  ).toBeVisible();
  await expect(page.getByTestId("semantic-node")).toHaveCount(0);
  await expect(
    page.getByRole("link", { name: /legacy expert console/i }),
  ).toHaveAttribute("href", "/expert");
  await page.getByRole("button", { name: "Toggle color theme" }).click();
  await expect(page.locator(".studio-light")).toBeVisible();
});

test("Studio shared selection, semantic filters, explanation and revision binding", async ({
  page,
}) => {
  const requests = await installFixture(page);
  await page.goto("/studio");
  await expect(
    page.getByRole("heading", { name: "System World" }),
  ).toBeVisible();
  await expect(page.getByTestId("semantic-node")).toHaveCount(2);
  const transforms = await page
    .getByTestId("semantic-node")
    .evaluateAll((nodes) =>
      nodes.map((node) => node.getAttribute("transform")),
    );
  await page
    .getByRole("button", { name: "FixtureSystem, PartDefinition", exact: true })
    .click();
  await expect(
    page
      .locator(".studio-inspector")
      .getByRole("heading", { name: "FixtureSystem", exact: true }),
  ).toBeVisible();
  await page.screenshot({
    path: "verification/generated/agentique-studio-phase3/studio-contract.png",
  });
  const views = requests.filter((item) => item.path === "/view").length;
  await page.getByRole("button", { name: "Zoom in", exact: true }).click();
  await page.getByRole("button", { name: "Zoom out", exact: true }).click();
  expect(requests.filter((item) => item.path === "/view")).toHaveLength(views);
  expect(
    await page
      .getByTestId("semantic-node")
      .evaluateAll((nodes) =>
        nodes.map((node) => node.getAttribute("transform")),
      ),
  ).toEqual(transforms);
  await page
    .getByRole("navigation", { name: "Studio worlds" })
    .getByRole("button", { name: "Graph", exact: false })
    .click();
  await expect(page.getByTestId("semantic-edge")).toHaveCount(1);
  await page
    .getByRole("checkbox", { name: "Ownership", exact: true })
    .uncheck();
  await expect(page.getByTestId("semantic-edge")).toHaveCount(0);
  await page.getByRole("checkbox", { name: "Ownership", exact: true }).check();
  await page.getByTestId("semantic-edge").click();
  await page
    .getByRole("button", { name: "Why this relationship?", exact: false })
    .click();
  await expect(
    page.getByRole("dialog", { name: "Semantic explanation" }),
  ).toBeVisible();
  await expect(
    page.getByText("Fixture derivation", { exact: true }).first(),
  ).toBeVisible();
  expect(requests.find((item) => item.path === "/explain")?.body).toMatchObject(
    { revision: R2, element: relationship },
  );
  await page.getByRole("button", { name: "Close explanation" }).click();
  await page.getByRole("button", { name: "Compare parent" }).click();
  await expect(
    page.locator(`.studio-node.changed[data-element-id="${root}"]`),
  ).toBeVisible();
  await expect(page.locator(".studio-edge.changed")).toBeVisible();
  await page.getByRole("button", { name: "Close comparison" }).click();
  await page
    .getByRole("combobox", { name: "Revision", exact: true })
    .selectOption(R1);
  await expect(page.locator(".studio-inspector-empty")).toBeVisible();
  await expect
    .poll(
      () =>
        requests.filter((item) => item.path === "/view").at(-1)?.body.revision,
    )
    .toBe(R1);
  await page
    .getByRole("button", { name: "FixtureSystem, PartDefinition", exact: true })
    .click();
  await expect
    .poll(
      () =>
        requests.filter((item) => item.path === "/inspect").at(-1)?.body
          .revision,
    )
    .toBe(R1);
});

test("Agent view and explicit candidate validation, commit, history, and rejection", async ({
  page,
}) => {
  const requests = await installFixture(page);
  await page.goto("/studio");
  await page
    .getByRole("button", { name: "FixtureSystem, PartDefinition", exact: true })
    .click();
  await page
    .getByRole("navigation", { name: "Studio worlds" })
    .getByRole("button", { name: "Agents", exact: false })
    .click();
  await page.getByRole("button", { name: /Explore dependencies/ }).click();
  await expect(
    page.getByRole("heading", { name: "Graph World" }),
  ).toBeVisible();
  expect(requests.find((item) => item.path === "/agent")?.body).toMatchObject({
    revision: R2,
    selection: [root],
  });
  await page
    .getByRole("button", { name: "Create nested part", exact: false })
    .click();
  await page
    .getByRole("textbox", { name: "New part name" })
    .fill("redundantController");
  await page.getByRole("button", { name: "Prepare candidate" }).click();
  const dialog = page.getByRole("dialog", {
    name: "Review candidate revision",
  });
  await expect(dialog).toBeVisible();
  await expect(
    dialog.getByRole("button", { name: "Accept & commit" }),
  ).toBeDisabled();
  expect(
    requests.find((item) => item.path === "/candidates")?.body,
  ).toMatchObject({
    revision: R2,
    command: {
      kind: "CreatePartUsage",
      owner: root,
      name: "redundantController",
    },
  });
  await dialog.getByRole("button", { name: "Validate", exact: true }).click();
  await expect(
    dialog.getByRole("button", { name: "Accept & commit" }),
  ).toBeEnabled();
  await dialog.getByRole("button", { name: "Accept & commit" }).click();
  await expect(dialog).not.toBeVisible();
  await expect(
    page.getByRole("combobox", { name: "Revision", exact: true }),
  ).toHaveValue(R3);
  await expect(page.getByTestId("semantic-node")).toHaveCount(3);
  await page
    .getByRole("navigation", { name: "Studio worlds" })
    .getByRole("button", { name: "History", exact: false })
    .click();
  await expect(page.locator(".studio-revision-card")).toHaveCount(3);
  await page
    .getByRole("navigation", { name: "Studio worlds" })
    .getByRole("button", { name: "System", exact: false })
    .click();
  await page
    .getByRole("button", { name: "FixtureSystem, PartDefinition", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Create nested part", exact: false })
    .click();
  await page
    .getByRole("textbox", { name: "New part name" })
    .fill("anotherPart");
  await page.getByRole("button", { name: "Prepare candidate" }).click();
  await page.getByRole("button", { name: "Discard candidate" }).click();
  await expect(dialog).not.toBeVisible();
  expect(
    requests.some(
      (item) =>
        item.path === "/candidates/fixture-candidate" &&
        item.method === "DELETE",
    ),
  ).toBe(true);
});

test("Studio refuses a projection from the wrong immutable revision", async ({
  page,
}) => {
  await installFixture(page);
  await page.route("**/api/gen2/studio/view", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(projection(R1)),
    }),
  );
  await page.goto("/studio");
  await expect(page.getByRole("alert")).toContainText("Revision mismatch");
  await expect(page.getByTestId("semantic-node")).toHaveCount(0);
});
