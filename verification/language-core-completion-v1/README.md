# Language core completion v1 evidence

**LANGUAGE CORE STRUCTURAL FOUNDATION INCOMPLETE — DO NOT PROCEED**

Fetched `origin/main` and started from a clean worktree at
`76aef699ee7b2d4f910c7483e42340998770ac5c`, on new branch
`foundation/language-core-completion-v1`. No SysML runtime crate was emitted.
Kernel invariants, existing language descriptors/views, semantic rules, source
pins and supplied artifacts remain unchanged. No Pack 2B work was undertaken.

## Reproduction and investigation

[Original readiness result](stage-0/results.json): the documented command ran
before code changes and returned **1**, with the exact `2abb2284… -> 2e4efe58…`
redefinition failure. [Original audit](original-structural-audit.json) preserves
the prior report. No historical failed gate is removed or relabelled successful.
The original PowerShell log is UTF-16; a
[UTF-8 viewing copy](stage-0/runtime-readiness.utf8.txt) is also supplied.

[Property evidence](stage-0/property-evidence.json) records direct-XMI ownership,
contexts, types, associations, class/association ancestry, source-qualified IDs,
UUIDs, byte/line locations and relevant retained bodies. It records inspected PDF
pages and hashes, the final UML Property operation list and supporting OCL bodies.
The independent investigation confirms that final UML 2.5.1 has no declared
Property context-validity override. UML24-85's resolution summary discusses mixed
ownership with inheritance, but neither actual association has any generalization.

[ADR 0009](../../docs/adr/0009-property-redefinition-context.md) states the
Category F authority gap and exactly what external clarification is needed.
This is not an invariant correction, an automatic baseline anomaly waiver, or
a determination that all mixed-owner redefinitions are invalid. Gate 1 and the
later runtime implementation gates are uncompleted.

The supporting research script `property_evidence.py` uses standard-library XML
and pypdf against unchanged local inputs plus previously acquired UML PDF/XMI
under `.cache/sysml-resume/` and the issue page under
`.cache/language-core-completion-v1/`. The recorded source URLs identify those
research inputs; they are not build dependencies. The legacy issue text endpoint
returned HTTP 403. The web reader also could not access the issue's JIRA browse
and REST representations. Those failures do not prove what unavailable text says.

## Implemented diagnostic tooling

The generator's `--audit-full` inventories all 82 KerML and 93 SysML metaclasses,
715 properties, 319 binary associations, seven enumeration domains and 19
literals. It attempts full descriptor translation, which fails before atomic
registration. It records association inheritance, primitive-domain gaps,
five exact naming anomalies and the Property authority gap separately.

Complete manifests preserve raw facts, provenance and frozen descriptor IDs.
`--require-runtime` now checks complete input for **both** baselines; it cannot
certify full KerML readiness by registering only the historical 29-class slice.
The default `--check` also checks new reports. Report-currentness success is
not runtime readiness. The old closure report remains unchanged.

`direct_xmi.py` supplies the second verification path, using Python's XML parser,
Expat byte offsets and uuid5 rather than Rust import/generation helpers. It
checks complete identity sets and direct structural facts, original artifact
hashes and exact source ranges. See [independent results](independent-verification.json).
It does not independently validate effective inheritance or execute retained rules.

## Focused checks

[First focused run](focused/results.json): generator tests returned 101 because
an existing stale-audit fixture did not provision the newly required full
reports. The other three checks passed. The fixture now supplies those reports
so its intended historical-audit staleness assertion remains meaningful.
[Focused recheck](focused-recheck/results.json): all four commands returned 0.

The new regressions cover complete selection (including disconnected declarations),
preserved runtime IDs, cross-metamodel provenance, category/count evidence,
no runtime emission from audit mode, read-only stale-report refusal and failure
of both complete runtime gates. They do not stand in for the requested generic
Property-rule or runtime foundation stress matrix, which remains due.

## Final verification

Run from repository root:

```sh
node verification/language-core-completion-v1/run.mjs final
```

The append-only runner records complete stdout/stderr and actual exit codes,
sets `CARGO_NET_OFFLINE=true` and strict `RUSTDOCFLAGS=-D warnings`, and covers
every requested command plus independent verification and preservation. It
documents all five existing generation-2 public crates and the generator.
There is no new public runtime crate to document.

Standards/browser reports and screenshots are copied into the new evidence
directory and their previous original bytes restored. The preservation checker
verifies protected files against fetched Git content and against the captured
pre-final working bytes. Some older reports already have CRLF in the clean
checkout while Git stores LF; the checker reports these differences explicitly
and never rewrites the old files. Existing anomaly entries are checked unchanged.

The initial raw-Git-byte preservation experiment detected that pre-existing
line-ending difference in `verification/benchmark.json`; it did not establish
a content change. The revised checker captures current bytes only after proving
Git equivalence (allowing only CRLF in historical text evidence), then requires
exact byte equality across final verification.

All fourteen final commands ran: [results](final/results.json). Twelve returned
0. Both complete runtime-readiness commands returned **1**; the runner therefore
also returned **1**. All four browser tests passed, all five public generation-2
crates passed strict Rustdoc, and 393 protected files passed preservation checks.
These failures are actual gates, not converted into passing refusal tests.

Implementation/investigation commit: `c6a52a0`. A separate verification commit
records these final logs. The foundation review's
[13 answers](../../docs/language-core-foundation-review.md)
and [machine-readable coverage](../../standards/v2-coverage.json) remain
incomplete regardless of existing workspace test success.
