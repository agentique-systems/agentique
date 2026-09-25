# Agentique Studio

Studio is the first Generation 2 spatial operator surface. It reads immutable
durable model revisions, with System, Graph, Requirements, History, Agents and
Source lenses. The backend projects canonical IDs; the browser stores selection,
camera and presentation choices.

From a fresh checkout, fetch build dependencies and build the frontend once:

```powershell
npm ci
cargo fetch --locked
npm run build
cargo run --locked --offline -p agq-studio -- setup --bundle C:\path\to\accepted-runtime.agq-runtime
cargo run --locked --offline -p agq-studio
```

Open `http://127.0.0.1:7332/studio`. The normal runtime store is `~/.agentique`;
the durable project defaults to `~/.agentique/projects/agentique.sqlite`.
`--runtime-dir <directory>` overrides the store and the default project directory;
`--database <file>` selects a separate durable repository. These paths do not
depend on `verification/generated`, `.workspaces`, or an earlier Phase 2 database.

The setup command accepts a bundle directory or `.agq-runtime` archive. Both
publications must authenticate independently against the checked-in accepted
receipts before an atomic installation. Installation is also available through
the Studio setup screen: enter an absolute local bundle path and choose **Use
local bundle**. The screen reports actual authentication and opening phases;
there are no estimated percentages. Read/Propose agent credentials cannot install
or retry the runtime. Semantic endpoints remain unavailable until the real
repository is open.

An accepted bundle still has to be distributed: no release asset was available
during First Light implementation. The complete operator sequence remains
unaccepted until those existing accepted bytes are supplied. See the
[runtime bundle contract](runtime-publications.md) and
[distribution handoff](runtime-publication-distribution.md). Do not rebuild a
standards publication to satisfy startup. Network access is unnecessary after
build dependencies and an accepted bundle have been obtained.

Explicit `--bundle` or paired `--kerml-cache` / `--systems-cache` options remain
available for development. `AGENTIQUE_RUNTIME_DIR` and legacy cache environment
variables are compatibility inputs. Normal startup discovers the installed
bundle, authenticates it, and never downloads or republishes standards.

An empty database is seeded from `models/agentique`: the five-document architecture
is committed and retained on `architecture-baseline`, then the ordinary future
Agent Fabric concepts are added on `main`. Both commits require actual validation.
An existing authored repository is not overwritten, including an intentionally
emptied project. A durable bootstrap intent journal resumes an interrupted first
import only when its acknowledged source contents and parent revision match.
Saved view definitions are
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

`GET /api/gen2/studio/runtime` reports separate measured phases for locating,
KerML/SysML authentication, repository open, initial architecture validation,
Agent Fabric validation, and durable semantic restore. The host logs publication
transport/source checks separately from cache restoration. Phase completion is
an observed transition, not a claim that pending semantic answers are Complete.

Run the real-host browser gate with `npm run test:studio`. Its missing-runtime
case creates fresh state, checks the setup screen, rejected installation and
agent authority, and saves `studio-runtime-setup.png`. That image establishes
setup behavior only. Supply `AGENTIQUE_STUDIO_BUNDLE` pointing at an existing
accepted bundle to enable the separate full operator sequence, including real
views/evidence, history/diff, agent view, candidate validation/commit and process
restart. It intercepts no semantic endpoints. The real semantic screenshot is
`verification/generated/agentique-studio-first-light/agentique-modeling-agentique.png`;
it is created only by that successful real-model path. Logs and measured timings
are retained next to it, while each run records its unique fresh database path.

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
