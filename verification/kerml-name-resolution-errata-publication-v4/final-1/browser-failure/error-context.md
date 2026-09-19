# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: console.spec.ts >> Console lifecycle, source navigation, shared context and reviewed Assistant change
- Location: tests\browser\console.spec.ts:143:1

# Error details

```
Error: expect(locator).toBeEnabled() failed

Locator:  getByRole('button', { name: 'Initialise', exact: true })
Expected: enabled
Received: disabled
Timeout:  5000ms

Call log:
  - Expect "toBeEnabled" getByRole('button', { name: 'Initialise', exact: true }) with timeout 5000ms
  - waiting for getByRole('button', { name: 'Initialise', exact: true })
    13 × locator resolved to <button disabled>Initialise</button>
       - unexpected value "disabled"

```

```yaml
- button "Initialise" [disabled]
```

# Test source

```ts
  124 |     page.getByRole("button", { name: "Initialise", exact: true }),
  125 |   ).toBeEnabled();
  126 |   await page.getByRole("button", { name: "Initialise", exact: true }).click();
  127 |   await page.getByRole("button", { name: "Run", exact: true }).click();
  128 |   await expect(page.locator(".run-stats")).toContainText("input_exhausted", {
  129 |     timeout: 45000,
  130 |   });
  131 |   await page
  132 |     .getByRole("button", { name: "Next trace page", exact: true })
  133 |     .click();
  134 |   await expect(
  135 |     page.getByRole("navigation", { name: "Trace pages" }),
  136 |   ).toContainText("1001–1023 of 1023");
  137 |   await expect(page.locator(".experiment tbody tr")).toHaveCount(23);
  138 |   await page
  139 |     .getByRole("button", { name: "Previous trace page", exact: true })
  140 |     .click();
  141 |   await expect(page.locator(".experiment tbody tr")).toHaveCount(1000);
  142 | });
  143 | test("Console lifecycle, source navigation, shared context and reviewed Assistant change", async ({
  144 |   page,
  145 |   request,
  146 | }) => {
  147 |   const errors: string[] = [];
  148 |   page.on("pageerror", (e) => errors.push(e.message));
  149 |   await page.goto("/#token=" + token);
  150 |   await expect(
  151 |     page.getByRole("heading", { name: "A model we can run." }),
  152 |   ).toBeVisible();
  153 |   await expect(page.getByText("TEST MODE", { exact: true })).toBeVisible();
  154 |   const initial = await (
  155 |     await request.get("/api/agentique/state", { headers })
  156 |   ).json();
  157 |   await page
  158 |     .getByRole("button", { name: "Rearrange views", exact: true })
  159 |     .click();
  160 |   const afterLayout = await (
  161 |     await request.get("/api/agentique/state", { headers })
  162 |   ).json();
  163 |   expect(initial.model.source_digest).toBe(afterLayout.model.source_digest);
  164 |   await page
  165 |     .getByRole("button", { name: "Rearrange views", exact: true })
  166 |     .click();
  167 |   await page
  168 |     .getByRole("button", { name: "Prepare new run", exact: true })
  169 |     .click();
  170 |   await expect(
  171 |     page.getByRole("button", { name: "Initialise", exact: true }),
  172 |   ).toBeEnabled();
  173 |   await page.getByRole("button", { name: "Initialise", exact: true }).click();
  174 |   await expect(page.locator(".run-stats")).toContainText("idle");
  175 |   await page.getByRole("button", { name: "Step", exact: true }).click();
  176 |   await expect(page.locator(".run-stats")).toContainText("checking");
  177 |   await page.getByRole("button", { name: "Step", exact: true }).click();
  178 |   await expect(page.locator(".run-stats")).toContainText("accepted");
  179 |   await expect(page.locator(".run-stats")).toContainText("condition_met");
  180 |   await page
  181 |     .locator(".experiment")
  182 |     .getByRole("button", { name: "acceptCheck", exact: true })
  183 |     .click();
  184 |   await expect(page.locator(".properties h2")).toHaveText("acceptCheck");
  185 |   await expect(
  186 |     page.getByRole("heading", { name: "RequestLifecycle", exact: true }),
  187 |   ).toBeVisible();
  188 |   await page.getByLabel("Ask about this model").fill("inspect");
  189 |   await page.getByRole("button", { name: "Send", exact: false }).click();
  190 |   await expect(page.locator(".message.assistant").last()).toContainText(
  191 |     "acceptCheck",
  192 |   );
  193 |   await page.getByLabel("Ask about this model").fill("rename acceptValidated");
  194 |   await page.getByRole("button", { name: "Send", exact: false }).click();
  195 |   await expect(
  196 |     page.getByRole("button", { name: "Approve and execute", exact: true }),
  197 |   ).toBeVisible();
  198 |   await page
  199 |     .getByRole("button", { name: "Approve and execute", exact: true })
  200 |     .click();
  201 |   await expect(page.locator(".properties h2")).toHaveText("acceptValidated");
  202 |   await expect(page.locator(".pin")).toContainText("Older than accepted model");
  203 |   await page.locator(".topbar").scrollIntoViewIfNeeded();
  204 |   await page.screenshot({
  205 |     path: "verification/screenshots/console-desktop.png",
  206 |     fullPage: true,
  207 |   });
  208 |   const updated = await (
  209 |     await request.get("/api/agentique/state", { headers })
  210 |   ).json();
  211 |   expect(updated.head_revision_id).not.toBe(initial.head_revision_id);
  212 |   const before = initial.model.elements.find(
  213 |     (e: any) => e.name === "acceptCheck",
  214 |   );
  215 |   expect(
  216 |     updated.model.elements.find((e: any) => e.name === "acceptValidated").id,
  217 |   ).toBe(before.id);
  218 |   await page.getByRole("button", { name: "rejected", exact: true }).click();
  219 |   await page
  220 |     .getByRole("button", { name: "Prepare new run", exact: true })
  221 |     .click();
  222 |   await expect(
  223 |     page.getByRole("button", { name: "Initialise", exact: true }),
> 224 |   ).toBeEnabled();
      |     ^ Error: expect(locator).toBeEnabled() failed
  225 |   await page.getByRole("button", { name: "Initialise", exact: true }).click();
  226 |   await page.getByRole("button", { name: "Run", exact: true }).click();
  227 |   await expect(page.locator(".run-stats")).toContainText("rejected");
  228 |   await page.getByRole("button", { name: "exhausted", exact: true }).click();
  229 |   await page
  230 |     .getByRole("button", { name: "Prepare new run", exact: true })
  231 |     .click();
  232 |   await page.getByRole("button", { name: "Initialise", exact: true }).click();
  233 |   await page.getByRole("button", { name: "Run", exact: true }).click();
  234 |   await expect(page.locator(".run-stats")).toContainText("input_exhausted");
  235 |   await page.setViewportSize({ width: 390, height: 844 });
  236 |   await page.screenshot({
  237 |     path: "verification/screenshots/console-mobile.png",
  238 |     fullPage: true,
  239 |   });
  240 |   expect(errors).toEqual([]);
  241 | });
  242 | test("API authentication, scope, identity mapping and revision checks", async ({
  243 |   request,
  244 | }) => {
  245 |   expect((await request.get("/api/agentique/state")).status()).toBe(401);
  246 |   const state = await (
  247 |     await request.get("/api/agentique/state", { headers })
  248 |   ).json();
  249 |   const projects = await (
  250 |     await request.get("/api/model/projects", { headers })
  251 |   ).json();
  252 |   expect(projects[0]["@id"]).toBe(state.project_id);
  253 |   const schema = JSON.parse(
  254 |     fs.readFileSync("standards/artifacts/Schema.json", "utf8"),
  255 |   );
  256 |   const branchId = projects[0].defaultBranch["@id"];
  257 |   const branch = await (
  258 |     await request.get(
  259 |       `/api/model/projects/${state.project_id}/branches/${branchId}`,
  260 |       { headers },
  261 |     )
  262 |   ).json();
  263 |   const commit = await (
  264 |     await request.get(
  265 |       `/api/model/projects/${state.project_id}/commits/${state.head_revision_id}`,
  266 |       { headers },
  267 |     )
  268 |   ).json();
  269 |   for (const [name, data] of [
  270 |     ["Project", projects[0]],
  271 |     ["Branch", branch],
  272 |     ["Commit", commit],
  273 |   ] as const) {
  274 |     expect(Object.keys(data).sort()).toEqual(
  275 |       Object.keys(schema.$defs[name].properties).sort(),
  276 |     );
  277 |     for (const required of schema.$defs[name].required)
  278 |       expect(data).toHaveProperty(required);
  279 |   }
  280 |   const first = await request.get(
  281 |     `/api/model/projects/${state.project_id}/commits/${state.head_revision_id}/elements?excludeUsed=true&page[size]=2`,
  282 |     { headers },
  283 |   );
  284 |   const firstData = await first.json();
  285 |   expect(firstData).toHaveLength(2);
  286 |   const next = first.headers().link.match(/<([^>]+)>; rel="next"/)?.[1];
  287 |   expect(next).toBeTruthy();
  288 |   const second = await (await request.get(next!, { headers })).json();
  289 |   expect(second[0]["@id"]).not.toBe(firstData[0]["@id"]);
  290 |   const elements = await (
  291 |     await request.get(
  292 |       `/api/model/projects/${state.project_id}/commits/${state.head_revision_id}/elements`,
  293 |       { headers },
  294 |     )
  295 |   ).json();
  296 |   expect(elements[0]["@id"]).toBeTruthy();
  297 |   expect(
  298 |     (await request.get("/api/model/projects/wrong", { headers })).status(),
  299 |   ).toBe(404);
  300 |   const stale = await request.post("/api/agentique/commands", {
  301 |     headers,
  302 |     data: {
  303 |       command_id: crypto.randomUUID(),
  304 |       project_id: state.project_id,
  305 |       base_revision_id: "stale",
  306 |       payload: { op: "save_draft", file: "a.sysml", source: "package A;" },
  307 |     },
  308 |   });
  309 |   expect(stale.status()).toBe(409);
  310 |   const forged = await request.post("/api/agentique/commands", {
  311 |     headers,
  312 |     data: {
  313 |       command_id: crypto.randomUUID(),
  314 |       project_id: state.project_id,
  315 |       base_revision_id: state.head_revision_id,
  316 |       actor: "operator",
  317 |       payload: { op: "save_draft", file: "a.sysml", source: "package A;" },
  318 |     },
  319 |   });
  320 |   expect(forged.ok()).toBe(false);
  321 |   const events = await (
  322 |     await request.get("/api/agentique/events?after=0&limit=1000", { headers })
  323 |   ).json();
  324 |   expect(
```