# SysML 2.0 pinned inputs

Authority: [OMG formal SysML 2.0](https://www.omg.org/spec/SysML/2.0), language
document formal/26-03-02. Preliminary SysML 2.1 / KerML 1.1 are not inputs.
[lock.json](lock.json) records exact provenance, SHA-256, length, role,
representation, normative status and acquisition time. Original bytes are preserved.

| Artifact | OMG file ID | SHA-256 | Bytes |
| --- | --- | --- | --- |
| SysML.xmi | ptc/25-02-15 | `caa65d54f56798bf7582d173f7567e1eea37a49c45984f8bd7df145011cf8c6f` | 1135635 |
| SysML.json | ptc/25-04-32 | `bb0d8af159cf2cbe4a0df4ed6b903505a57e33d047068e5a50a7008a18d546c5` | 3875821 |
| Systems-Library.kpar | ptc/25-04-24 | `df7d8b2c6e08232ca7ce123a63148949c383fcbeaeba8d89c27ceece43793a1f` | 27577 |

XMI is the primary MOF authority. JSON is a representation cross-check, distinct
from the Systems Modeling API schema. The downloaded Systems Library matches
`standards/artifacts/Systems-Library.kpar` byte-for-byte; it is referenced there,
not duplicated. The three Kernel KPAR dependencies were also re-downloaded and
matched against the existing original bytes. No historical artifact was replaced.

The informative Simple Vehicle Model (ptc/25-04-31) is listed separately in the
lock as **unavailable**. Its publication link was inspected, but direct acquisition
returned HTTP 403 and web retrieval returned an HTML document-not-available page.
Neither that page nor a different example is presented as the official fixture.
It is not a normative dependency or a prerequisite for an authored acceptance model.

## Library content identity

[library-set.json](library-set.json) records the exact archive-level dependency
closure and every contained entry hash. The selected set is the publication-linked
original Systems Library plus Semantic, Data Type and Function libraries. Metadata
requires all three Kernel archives; their usage graph is cyclic and traversed with
a visited set. No optional SysML domain archives are selected.

Set identity is `sha256:6cceb50286d6edd411f327201b6016b5651744d4a30175c78e019810d482e928`.
Specification versions (KerML 1.0 / SysML 2.0), project versions (1.0.0 / 2.0.0),
correction publication and content hashes are distinct. The April 2026 correction
release already used by generation 1 has different bytes despite identical project
versions; it remains separately preserved in `standards/lock.json`. This choice
does not change the generation-1 library baseline.

KPARs contain ZIP-wrapped project metadata and textual `.kerml`/`.sysml` sources,
not JSON/XMI semantic model instances. The original Systems index points
`AnalysisCases` to the absent `AnalysisCase.sysml`; actual content is
`AnalysisCases.sysml`. Preserve and diagnose this discrepancy, never rewrite the
archive. Archive closure does not imply ingestion or semantic completeness;
dependency-closed textual loading remains Stage 4 work.

## Reproduction

`node tools/pin-sysml.mjs` is the explicit network maintenance command. It verifies
all upstream and existing local hashes before exclusive creation of any missing
input. It never replaces files or updates locks. `npm run standards:check` and
`node --test tools/sysml-artifacts.test.mjs` verify offline. Builds and tests do not
download standards. The KerML metamodel lock and all original inputs remain intact.
