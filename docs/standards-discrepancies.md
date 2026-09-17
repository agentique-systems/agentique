# Standards baseline and discrepancies

The supplied PDFs were identified from their title pages, not filenames:

| File | Actual publication | Version | OMG document |
|---|---|---|---|
| KerML.pdf | Kernel Modeling Language (KerML) | 1.0, March 2026 | formal/2026-03-01 |
| SysML.pdf | OMG Systems Modeling Language, Part 1: Language Specification | 2.0, March 2026 | formal/2026-03-02 |
| SysAPI.pdf | Systems Modeling Application Programming Interface (API) and Services | 1.0, March 2026 | formal/2026-03-04 |

[baseline-lock.json](../standards/baseline-lock.json) records original byte hashes, provenance URLs,
PDF title pages and downloaded official artifact entries. The four official
library projects and all eight declared dependency edges were checked against
their actual `.project.json` contents; see
[integrity evidence](../verification/standards-integrity.json). The cycle among
KerML semantic, function and datatype projects is a declared dependency cycle,
not an invented replacement library. All 57 textual library files are preserved.
The build fails if a library file differs from its locked hash.

The HTML is the scope/contract authority. OMG is the language meaning authority.
These observed inconsistencies are retained in the baseline, not silently edited:

1. SysML Table 32 names `Attributes::attributes`; the specific AttributeUsage
   constraint in §8.3.7 and semantics §8.4.3.2 use `Base::dataValues`, aliased by
   `Attributes::attributeValues` in the published library. Agentique uses the
   specific constraint and the actual declaration.
2. Several illustrative/table paths use `States::State` or `State::states`;
   the specific rules and current library use `States::StateAction`,
   `stateActions`, and `StateAction::exclusiveStates`. Agentique uses those real
   declarations. Similarly, ExhibitStateUsage prose contains `exhibitStates`
   while the library and explicit example use `exhibitedStates`.
3. SysML §8.4.13.6 explicitly excludes a transition trigger's AcceptActionUsage
   from standalone `Actions::acceptActions` specialization. Agentique maps the
   trigger through `Actions::TransitionAction::accepter`, consistent with
   §8.4.14.3 and the actual library's AcceptMessageAction typing.
4. All four supplied official `.kpar` downloads wrap their files in a project
   directory; KerML §10.3 describes project metadata at the archive top level.
   Acquisition preserves the archives and extracts the wrapper explicitly.
   Agentique's own exports use top-level metadata. Its bounded project importer
   requires that form and does not claim to ingest the complete official libraries
   as editable user projects.
5. Systems Library `.meta.json` maps `AnalysisCases` to `AnalysisCase.sysml`,
   whereas the archive contains `AnalysisCases.sysml`. No official file was
   patched. The declaration index reads all verified source entries. This metadata
   defect is recorded by the integrity check.
6. The API JSON schemas obtained from the 20250201 publication still contain
   20230201 `$id`/reference URIs. The bytes and their identifiers remain unchanged;
   this is not interpreted as a new private API standard.

Independent validation used **sysml-validate 0.43.1**, an external implementation,
with the actual pinned library directory and no ambient configuration. The four
starter models returned zero errors and warnings, with twenty style hints. No
demonstrable starter defect was found, so their bytes remain exactly as extracted.
[The validator result](../verification/independent-validation.json) is an external
check of the starter, not proof that every Engine semantic transformation conforms.

The independent Controller fixture is derived from SysML §7.18.3's OnOff examples
and §8.4.14.3's transition payload rule. The external validator reports an unresolved
`r.count` in the guard of a named payload transition. The specific rule says the
outer transition payload subsets the trigger payload and inherits its effective
name (printed pp. 441–442). Agentique resolves that parameter and has an exact
large-integer guard test. The fixture and external diagnostic are both retained in
[independent-fixtures.json](../verification/independent-fixtures.json). This
disagreement was initially unresolved. The subsequent official pilot check below
resolves the payload question; the npm check is still not reported as passing,
and no fixture was renamed to hide it.

The published Java pilot **0.59.0 (2026-04)** was subsequently acquired using the
conda-forge package selected by the official installation instructions, with
archive and runtime provenance in [pilot-provenance.json](../verification/pilot-provenance.json).
Its `CheckMode.ALL` check against the original libraries exposed twelve errors:
six incompatible transition payload redefinitions and six overriding receiver
bindings. These are preserved in [pilot-baseline-validation.json](../verification/pilot-baseline-validation.json).
The original `States::StateTransitionAction::payload` is `out`, conflicting with
the input parameter in SysML §8.4.14.3. The original
`TransitionPerformances::TransitionPerformance::accept::receiver` has a fixed
binding that conflicts with a transition's explicit `via` value.

The official [2026-04 library release](https://github.com/Systems-Modeling/SysML-v2-Release/tree/9baca5908ca28b53da085de69336fde48420ea8f/sysml.library.kpar)
corrects these declarations: payload is `inout`, permitting the specified `in`
redefinition, and the receiver constraint uses a binding connector. Agentique
now selects those four official `.kpar` projects in [lock.json](../standards/lock.json),
with immutable commit URLs, archive/entry hashes, original dependency resource
identifiers and exact library versions (KerML libraries 1.0.0, Systems 2.0.0).
`tools/library-release.mjs` reproduces the selection. Original downloads and
extracted files remain intact; no standard-library source was patched. The new
archives also correct the wrapped project root and AnalysisCases index defects.

With the selected libraries, the pilot reports **zero errors and two warnings**
across all four starter models and Controller. It resolves `r.count`; this gives
independent evidence for the specific payload rule and establishes that the npm
diagnostic is a validator limitation, not a fixture correction to apply. The
raw npm failure remains recorded and is not counted as passing. See
[pilot-validation.json](../verification/pilot-validation.json). The two warnings
concern the starter's local state `accepted` sharing an inherited member name;
the pilot does not classify them as errors. The starter bytes remain unchanged.

These checks are independent implementation checks, not an OMG conformance-suite
certification of Agentique's entire semantic graph. The provider transport is
exercised against a local HTTP fixture; no real credentialed service was available.
