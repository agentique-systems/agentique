# Graph review round 3: independent relationship endpoints

The independent graph review found that satisfies/verifies arrows in Requirements
World converged on the same final segment, preventing individual traceability.
The integration captures in `round-02-after/04-requirements-world.png` are the
before evidence; the integration stream owns the subsequent actual native
surface capture with the combined product changes.

Ordinary node boundaries now allocate deterministic attachment positions per
relationship, ordered by neighbor position and then exact relationship identity.
Distinct nearby turns preserve the final lanes. These positions are presentation
geometry, introduce no `ScenePort`, and do not change source/target identities,
direction, relationship family or revision. Actual modeled ports retain their
exact attachment coordinates even when several relationships share a port.

The first implementation staggered turns by up to 40 world units. Adversarial
cycle fixtures exposed 60 obstructed routes because long stubs crossed the next
unrelated card in a narrow gutter. The final implementation limits staggering to
6 world units beyond the existing 22-unit stub. The rerun restores zero obstructed
routes in both 80-node cycle layouts, now guarded by a regression test.

Qualification includes selecting each satisfies/verifies relationship separately
ten world units from its own target with a three-unit hit tolerance; distinct
parallel ordinary-node endpoints; unchanged modeled-port attachments; and
deterministic output after reversing edge input order. Eighteen adversarial
layout scenarios retain zero sibling overlaps and containment failures.

This does not remove every non-junction orthogonal crossing or create bridge
glyphs. Very dense fan-in can exhaust a finite card boundary; the renderer still
needs selected-path emphasis and individual relationship inspection. No claim of
semantic junction or compatibility follows from attachment positions.

| Command | Output | Exit |
|---|---|---|
| `cargo fmt --all` | no output | 0 |
| `cargo test --locked --offline -j 2 -p agq-studio-scene` | `routing-tests.txt` | 0 |
| `cargo clippy --locked --offline -j 2 -p agq-studio-scene --all-targets -- -D warnings` | `routing-clippy.txt` | 0 |
| `cargo run --locked --offline -j 2 -p agq-studio-scene --example layout_quality` | `routing-layout-quality.txt` | 0 |

All scenarios use explicit visual fixtures. Debug timings under parallel mission
work are diagnostic; native interaction and release performance are separate
integration gates.
