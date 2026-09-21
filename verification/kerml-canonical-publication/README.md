# KerML canonical publication closure

Canonical publication remains **incomplete**. KerML conformance coverage remains
**incomplete**, independently. This milestone implements Operational v8 and
focused graph corrections; it does not issue an accepted library facade.

Base: `5279d879534e1b56934b733ba638e14c90168db1`, fetched `origin/main` with a clean
worktree. Branch: `foundation/kerml-canonical-publication-closure`.

## Implemented

- Two independently pinned v8 authority manifests extend the frozen v7 profile.
  `SemanticContextId` includes both digests. Published through v7 keep their cross
  selector/domain and imported-Membership interpretation boundaries.
- Owned-cross domains exclude the owning end in v8. Binary domains use the
  opposite end's effective Types; n-ary domains preserve all other ends as
  Cartesian factors and intersect multiple Types within a factor. Required
  featuring/typing and inherited cross obligations retain provenance. Missing
  inherited crossing structure for a binary/n-ary general is explicitly
  incomplete. A fully known unary population stays determinate without invented
  opposite ends; its validation-only cardinality conflict remains visible.
- The generic namespace layer discovers and deduplicates canonical Memberships,
  excludes owned collisions, then excludes imported collisions symmetrically.
  It preserves noncolliding import order, aliases, visibility and cycle handling.
- Standard bindings carry per-role LibraryIds and a library-set identity. New
  roles replace v8 producer path searches for `Base::things::that`,
  `Occurrences::Occurrence::startShot`, and `Collections::Array`. The last role
  requires the pinned Data Type Library. Exact paths, metaclasses, uniqueness,
  visibility and declaration/slot provenance remain checked.
- Positional parameter projection includes inherited feature-chain targets;
  renamed local parameters suppress the inherited Membership identities.
- Negative formal owner-type antecedents require both complete effective typing
  and evidence that relevant type-producing antecedents are closed. A completed
  query over a partial graph alone is insufficient.
- Multiplicity bounds expose canonical ordered expression identities, without
  claiming numeric evaluation or creating duplicate expressions.

## Acceptance boundary

The [capability matrix](publication-capabilities.json) is the publication checklist.
The two former graph-affecting authority conflicts are
`CoveredByOperationalV8`; [authority decisions](authority-decisions.json) retain
KERML11-2 and KERML11-4 as validation-only conflicts. Neither conflict nor missing
ValidatorOnly coverage controls the publication result.

Required `CrossSubsetting` production still needs canonical derived association
occurrences: the current kernel occurrence record carries declared provenance.
Variable snapshot domains and some expression/reference binding contexts also
need producer closure. The existing overlay therefore remains
`PartialDerivationOverlay`. No `CompletePublicationOverlay`, accepted bindings,
canonical facade, or authored-project consumption is asserted.

The full three-library expansion is held until focused gates and all capability
families are complete. Source refinement across the pinned dependency set is
part of the focused test; only Links, Observation and Triggers documents are
expanded there. It is not the complete expanded publication gate.

## Focused measurements

The final focused run has 29,140 source records and 948 produced records over
three pinned KerML library artifacts. All 4,000 source endpoints are populated
and ordinary kernel construction obligations are zero. Semantic resolution was
rechecked for the ten retained regressions: all are Complete, unique, and agree
with their stored endpoints and required metaclasses. This is not a claim that
all 4,000 references have passed an expanded semantic audit.

Both retained Triggers namespaces contain 103 effective Memberships and zero
distinguishability findings. SelfLink uses its canonical opposite-end Type, and
all five VectorFunctions/NumericalFunctions collision witnesses pass: v7 retains
the imported Memberships and v8 excludes them while preserving owned identities.
The local standard-binding contract validates 27 roles. Nine capability families
still require closure; the full expanded publication gate therefore remains held.

## Reproduce

Run from the repository root, with the original locally retained pinned artifacts:

```powershell
python verification/kerml-canonical-publication/scripts/authority.py
cargo test --offline -p agq-kerml --test profile_lineage_publication_v8
cargo test --offline -p agq-kerml-semantics --test publication_v8_imports --test publication_v8_cross_domains --test publication_v8_members --test publication_v8_bounds
cargo test --offline -p agq-kerml-text --test library_construction
cargo run --locked --offline -p agq-kerml-text --example binding_manifest -- --check
python verification/scripts/run.py --name publication-focus --summary verification/kerml-canonical-publication/summary.json -- cargo run --release --locked --offline -p agq-kerml-text --example publication_focus
python verification/kerml-canonical-publication/scripts/report.py publication
python verification/kerml-canonical-publication/scripts/report.py conformance
python verification/kerml-canonical-publication/scripts/verify.py all
```

The historical binding stale check and publication reporter exit nonzero while
their acceptance gates remain incomplete. The
conformance reporter's exit zero means a report was generated, not conformance
success. Its scope and both validation-only authority conflicts remain explicit.

[summary.json](summary.json) records actual commands, exit codes, output digests,
tool versions, working-tree identities and scoped results. All raw command and
query output is ignored under `verification/generated/`. The first workspace
test build exhausted disk space while generating Windows debug symbols; native
Cargo package cleanup and a retry with incremental compilation disabled are
recorded separately. No source artifact was removed or modified.

The retained `/1` binding manifest contains 22 historical anchor roles; the code's
`/3` binding contract now validates 27. The stale check correctly fails; no roles
are omitted from that check to make it pass. Regeneration/version increment
awaits accepted canonical IDs. The new role contracts have separate positive and
negative construction tests. Generation-1 release records and SysML language
support are unchanged.
