# Studio frontend verification

The visible application now defaults to Gen2 Studio at `/` and `/studio`.
The existing Gen1 console remains at `/expert`; its browser test navigation was
updated without changing its assertions. Vite routes `/api/gen2` to the separate
Studio host on port 7332 and the existing API to port 7331.

Final focused frontend commands passed:

| Command | Exit | Result |
| --- | ---: | --- |
| `npm run check` | 0 | TypeScript application contracts |
| `npm run build` | 0 | Production Vite build |
| `npx playwright test --config playwright.studio.config.ts` | 0 | 4 browser transport-contract tests, 5.7 seconds |

`frontend-commands.json` records actual arguments, versions, working tree identity,
duration, output hashes, and all attempts. Raw transcripts and screenshots live
under ignored `verification/generated/agentique-studio-phase3/`.

The four browser tests establish:

1. An unavailable repository produces an explicit connection state and no model
   objects. Theme switching and the expert route remain accessible.
2. SVG and outliner selection share semantic IDs with the inspector. Relationship
   toggles change the loaded projection without semantic reconstruction. Zoom
   causes no projection requests or layout mutation. Derived edge explanation
   targets the canonical relationship identity. Canonical diff changes highlight
   nodes and relationships even when changed properties are absent from the
   display DTO. Revision switching binds subsequent queries to the selected ID.
3. An agent result becomes a semantic view. CreatePartUsage produces an explicit
   Working candidate with source reconciliation. Commit is disabled before
   validation. Successful commit reloads the selected revision and history.
   Discarding a candidate releases it through the backend DELETE contract.
4. A projection carrying a different revision is rejected before rendering.

These tests intercept HTTP with an explicitly named **Contract fixture**. They
test the real React application against its transport contract, not actual
semantic reconstruction, publication authentication, repository durability, or
real agent execution. The generated `studio-contract.png` is an interaction
fixture screenshot, never a screenshot of the real Agentique self-model.

An initial `npm test` run passed 19/20 tests and failed the accepted Systems
publication implementation-input freshness check while the performance workstream
was changing construction code. That attempt remains recorded. The integration
lead owns the final reviewed freshness and workspace regression runs; a frontend
check does not establish a language gate.

The separate real durable Studio/browser sequence remains blocked by absent
accepted cache inputs, as recorded in `phase2-gates.md`. Interactive latency on
the actual self-model has not been measured. No standard publication was rebuilt.
