# Agentique Studio

Studio is the first Generation 2 spatial operator surface. It reads immutable
durable model revisions, with System, Graph, Requirements, History, Agents and
Source lenses. The backend projects canonical IDs; the browser stores selection,
camera and presentation choices.

Build and launch from the repository root:

```powershell
npm run build
$env:AGENTIQUE_KERML_CACHE = 'C:\path\to\accepted-kerml-cache.zip'
$env:AGENTIQUE_SYSTEMS_CACHE = 'C:\path\to\accepted-systems-cache.zip'
cargo run --locked --offline -p agq-studio -- --database .workspaces/studio.sqlite
```

Open `http://127.0.0.1:7332/studio`. Existing accepted cache files are required;
their actual filenames can differ. They must authenticate to the pinned accepted
publications. No download or publication build is performed by startup. A missing
cache leaves a visible setup state and HTTP 503, never a fabricated model.

An empty database is seeded from `models/agentique`: the five-document architecture
is committed and retained on `architecture-baseline`, then the ordinary future
Agent Fabric concepts are added on `main`. Both commits require actual validation.
An existing authored repository is not overwritten. Saved view definitions are
kept in an adjacent `.views.sqlite` platform metadata database.

Select a part to inspect its semantic data. Double-click or Focus to narrow the
world. Zoom changes display detail; relationship toggles, pan and zoom do no model
reconstruction. Graph edges select their actual relationships. Why displays
existing producer evidence. History changes the immutable revision used by the
viewport and inspector; comparisons label both revisions.

The deterministic agent can respond to “Show its dependencies” with a focused
semantic view, or “Show what changed” with a revision diff. It does not call an LLM.
Create a nested part from the inspector, review its Working candidate and exact
source change, validate, then commit. Until commit, the branch remains unchanged.
Uncommitted review handles do not survive process restart.

For frontend development, run the host and `npm run dev`; Vite proxies Studio
requests to port 7332 and the legacy API to port 7331. The legacy server still
serves the Generation 1 Console at `/expert`. Its existing approval and release
contracts are unchanged.

## Programmatic participants

Set `AGENTIQUE_STUDIO_AGENT_TOKEN` before launching and send
`Authorization: Bearer <token>` to `/api/gen2/studio`. The host assigns that
identity Read + Propose. Browser operator authority is separate. Do not put
credentials in checked-in configuration.

| Operation | Request |
| --- | --- |
| Discover durable project/history | `GET /session` |
| Project a lens | `POST /view` with `{project, revision, definition}` |
| Inspect or explain | `POST /inspect` or `/explain` with `{project, revision, element}` |
| Locate source | `POST /source` with the same binding |
| Compare revisions | `POST /diff` with `{project, from, to}` |
| Deterministic agent view | `POST /agent` with `{project, revision, selection, intent, compare_to?}` |
| Propose nested part | `POST /candidates` with `{project, branch, revision, command}` |
| Inspect candidate | `POST /candidates/{id}/inspect` with `{element}` |
| Explain candidate | `POST /candidates/{id}/explain` with `{element}` |
| Validate / commit | `POST /candidates/{id}/validate` or `/commit`; operator capability required |
| Reject | `DELETE /candidates/{id}` |
| Save a view definition | `POST /views` with `{project, definition}`; operator capability required |

The implemented command shape is
`{kind:"CreatePartUsage", owner:ElementId, name:string, definition:ElementId|null}`.
No arbitrary kernel mutation is exposed. The public Rust contracts are in
`agq-modeling-view` and `agq-modeling-agent`; existing Systems Modeling API
contracts remain in the separate API/HTTP adapters.

See [ADR 0029](adr/0029-agentique-studio-semantic-worlds.md) and the
[actual acceptance record](../verification/summaries/agentique-studio-phase3/README.md).
