# Real first-light visual review — systems engineer / graph critic

**Judgment: not yet suitable for a confident engineering demo.** The real data
exposes a serious gap between canonical correctness and visual comprehension.
This review inspected the actual rendered images, not only scenario assertions.

The images come from `real-run02`: authenticated KerML v9 / Systems v3, the
Agentique model, and revision `8dcfb224-55e6-4d0d-ac3a-84fcfffb9a46` with Complete
producer closure. The retained native scenario records the exact projection and
Inspector bindings. This is real semantic data, not the architecture fixture.

| Image inspected | SHA-256 |
| --- | --- |
| [01 — System World](../real-run02/gallery/01-system-world.png) | `6cae6a64c7bf76071719370d4ad837fe58ba22401d0ce635693d1f02516858a7` |
| [02 — ModelingPlatform focus](../real-run02/gallery/02-focused-subsystem.png) | `b7416f163e193e53a15b2068b34c73f1fc7e21412a4c7dc299b3070b22a58c9e` |
| [02a — clientQueries port](../real-run02/gallery/02a-platform-port.png) | `9f83795f81e1eb7c129d0589caaf97fdd4635983912717fcba446a7b36d45c24` |

## Ranked problems

1. **P1 — Anonymous architecture.** At the fitted 23% overview, almost every
   node is an unlabeled dark rectangle. Only ModelingPlatform's container title
   is readable. The canvas does not answer what Agentique contains or where an
   engineer should begin. A low-detail representation must still identify the
   principal systems; removing almost all names is not a useful semantic zoom.
2. **P1 — The system root is visually lost.** Thirty-eight records are spread in
   a grid with long empty corridors. The Explorer puts Agentique eighth, below
   KerMLEngine, ExecutionIR, ModelingService and other peer-looking records.
   Definition references and authored parts compete at the same apparent level.
   The project name in the top bar does not recover the missing architecture.
3. **P1 — Focusing does not feel like entering.** ModelingPlatform focus still
   shows 32 scene nodes at 24%; its own boundary remains a small box near the
   lower center. A principal user action has changed the projection without
   giving the operator a readable subsystem. Remote context anchors should not
   determine the useful reading scale of the entered subsystem.
4. **P1 — The engineering Inspector exposes semantic bookkeeping first.**
   ModelingPlatform and clientQueries show repeated inherited `Connector`
   entries and names such as `ownedPorts` and `interfacingPorts` under Ports &
   Interfaces. In the port view, these precede the actual connection. They are
   real semantic facts, but this default order recreates metamodel inspection.
   Authored inherited ports such as `Repository::repositoryRevisions` must
   remain discoverable without promoting unnamed derived and standard facts.
5. **P1 — Selection does not identify the port spatially.** The Inspector says
   clientQueries, but the canvas offers no readable endpoint marker or label at
   this scale. An engineer cannot confidently correlate the selected port with
   the surrounding architecture or distinguish it from its opposite endpoint.
6. **P2 — Relationships lack readable purpose.** Long thin, low-contrast
   orthogonal lines connect anonymous rectangles. At this scale neither the
   endpoint names nor ownership/typing roles are understandable. Showing every
   retained path adds visual complexity without communicating its identity.
7. **P2 — The initial Inspector uses space without orienting the operator.**
   “Everything has context” and navigation instructions occupy a substantial
   panel while the actual system is unreadable. Selecting the principal system
   and showing its components would be a more useful initial state.

## What the real interaction establishes

The clientQueries Inspector does show a useful real answer:
`queryConnection · ModelingPlatform.platformQueries`, with direction explicitly
“Not specified.” The captured canonical InterfaceUsage edge retains the actual
relationship ID `d5333e66-b345-5851-ba0e-30328505dd0c`, both port IDs, and authored
provenance. Its endpoint query reports Complete, with 1,202 positive and 1,637
search dependencies. This resolves the Inspector/scene semantic mismatch; it
does **not** make the barely legible endpoint depiction acceptable.

The scenario recorded 20,728 ms for selecting ModelingPlatform, 13,650 ms for
the keyboard focus step and 7,100 ms for the port inspection step. These are
whole automation-step durations, not isolated service time, input latency or
physical presentation latency. They warrant investigation alongside the
rendering critique. The first bound Validated view appeared about 233.6 seconds
after process start; that interval includes runtime and repository restoration.

## Requested iteration and re-review gate

First reduce the default architecture to one meaningful level, retaining a
small readable set of canonical system/part/definition identities. Put the
principal system first in the Explorer. Ensure the fitted result has readable
names; increasing the camera scale of the same 38-node projection is not enough.

On focus, make the entered subsystem the visual center and the target of camera
framing, with its components and named ports legible. Keep original canonical
owners and referenced definitions truthful; contextual elements can remain
available without forcing the camera to encompass the whole semantic graph.

Reorder the Inspector around project engineering objects and their real
connections. Move unnamed derived/standard inherited details behind Effective
semantics. Preserve the original identity and declaring owner of useful authored
inherited ports. Show a visible selected-port marker and its relationship path.

**Changes after this critique and new screenshots: pending.** Parent integration
has received these ranked findings. This record is the pre-iteration assessment;
it must not be cited as a completed visual improvement or alpha acceptance.
Re-review all three corresponding scenes after changes, rather than retaining
only the best overview image.
