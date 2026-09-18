# Language core completion v2 — Gate 0 evidence

**LANGUAGE CORE STRUCTURAL FOUNDATION INCOMPLETE — DO NOT PROCEED**

Fetched `origin/main`; the worktree was clean and HEAD matched
`a93f3e94630abc64a3ea16ede27abc22b1dd2d58`. Created
`foundation/language-core-completion-v2`. No commit was made directly to main.

[ADR 0010](../../docs/adr/0010-property-redefinition-validity.md) replaces the
prior acquisition question with a precise remaining authority conflict. All
three requested issue records were successfully acquired: UML24-85, UML24-86
and UML24-96 are closed/resolved. Their original HTML, URLs, lengths and SHA-256
hashes are in [evidence/](evidence/acquisition.json). There is no claim that these
records are unavailable or that an open issue prevents proceeding.

UML24-85 establishes that mixed ownership must be handled. UML24-96 separately
requires association ancestry for its association-owned cases. Final UML 2.5.1
retains the independent `redefined_property_inherited` constraint and does not
declare the proposed Property context-validity override. For the exact SysML
edge, the redefining association has no parents and Interaction does not inherit
Definition. Thus the final inherited-feature condition and both candidate
inheritance tests fail, although FlowUsage's value-type conformance passes.
Figure 22 repeats the source facts. No inspected resolution explains a different
context or inheritance relation for this edge.

This invokes the requested Category F stop condition at Gate 0. It does not
classify an ordinary missing generic capability as an authority ambiguity.
Gates 1–11 have not been implemented. In particular, the 16-row candidate matrix
is **not** the requested runtime valid/invalid test matrix, and the new evidence
checker is **not** the complete runtime stress matrix. No kernel correction,
complete typed views, primitive domains or SysML crate are claimed.

## Reproduction and evidence boundaries

`authority.py` independently reads the original language XMI through the prior
Python XML/Expat checker, with no Rust generator or kernel interpretation helpers.
It checks identities, ownership, exact relationships, both ancestry graphs,
source hashes and locations; verifies the three issue dispositions; reads final
UML operations/constraints; corroborates two association generalizations named
by UML24-96; and records PDF page hashes. The result is
[property-authority.json](evidence/property-authority.json).
`--check` compares bytes without writing. A zero exit means the evidence remains
reproducible, not that any candidate rule is accepted.

Research-only local prerequisites are Python `pypdf` and the previously acquired
`.cache/sysml-resume/UML.{pdf,xmi}` with hashes checked by the script. Their official
source URLs are in ADR 0010 and the evidence. PDF figures were rendered with
`pypdfium2` 5.13.0 installed only under `.cache/language-core-completion-v2/python`,
then visually inspected. This is not a build dependency or image modification.
No acquisition is part of a Cargo or npm build/test.

The [original foundation review](original-foundation-review.md) and
[original closure audit](original-structural-audit.json) preserve their prior
bytes. All earlier verification evidence, normative sources, generated artifacts,
runtime crates and the metamodel generator are protected against changes by
`preservation.mjs`. It checks Git content against the fetched base and exact
working bytes against [the initial capture](preservation-inputs.json). Pre-existing
CRLF checkout differences in historical reports are recorded, never rewritten.

## Actual checks

Run from the repository root, using a new label to retain any earlier attempt:

```text
node verification/language-core-completion-v2/run.mjs stage0
node verification/language-core-completion-v2/run.mjs focused
node verification/language-core-completion-v2/run.mjs final
```

The runner records exact command lines, merged stdout/stderr, exit codes and
timings, and sets `CARGO_NET_OFFLINE=true` and `RUSTDOCFLAGS=-D warnings`.
Historical standards/browser reports are copied into the new evidence directory
and restored at their old paths. It never converts a nonzero runtime gate into
success. The final runner's overall exit is nonzero if either runtime gate fails.

[Stage 0](stage0/results.json): complete audits are current; both complete runtime
commands returned **1**, stopping at the known association-inheritance translation
gap before atomic registration. The SysML report also lists the distinct Property
authority finding. The Property conflict is established by the independent source
analysis and preserved closure audit, not falsely attributed to that first
translation error.

[Focused checks](focused/results.json): all six commands returned **0**: authority
evidence, kernel tests, generator tests, formatting, complete audit currentness and
independent complete-XMI manifest verification. These are existing-runtime and
evidence checks; their success does not complete Gate 0.

The final verification result is recorded separately after execution. The
[foundation review](../../docs/language-core-foundation-review.md) must remain
incomplete unless both runtime gates and all foundation obligations pass.
