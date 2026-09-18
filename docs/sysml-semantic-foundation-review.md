# SysML semantic foundation review

Result: **INCOMPLETE — DO NOT START PLATFORM**.

This branch implements a generation-2 multi-document KerML project and an exact
pinned KPAR source loader. It does not complete the requested SysML semantic
foundation. The missing implementation is ordinary language work; this review
does not claim that an external authority or approval prevents continuing it.

The authority inventory covers all 93 SysML metaclasses and 384 directly owned
XMI rules/operations, including retained bodies, specification anchors, inherited
KerML obligations and library dependency references. It is not an interpreter of
those bodies or an exhaustive interpretation of specification prose. ADR 0012
defines the intended semantic ownership and initial structural family.

## Review questions

1. **Are standard KerML libraries loaded as canonical semantic elements?** No.
   Their three exact archives and 36 textual documents are verified and exposed
   as immutable source inputs. No canonical library declarations are constructed.
2. **Is the Systems Library loaded canonically?** No. Its archive and 21 source
   documents are verified, including its cyclic dependency closure. SysML parsing
   and semantic lowering remain unimplemented.
3. **Are library identities deterministic and provenance-preserving?** Source
   library/document/revision identities and the private element-allocation scheme
   are deterministic and tested against independent UUID goldens. Origins are
   `StandardLibrary` with the exact LibraryId. Repeatable canonical facts and
   semantic context identity have not been established because no library graph
   has been lowered. The immutable source inputs cannot be mutated through the
   verified loader API. The scheme is private Agentique policy, not OMG authority.
4. **Do mixed KerML/SysML projects resolve correctly?** Multiple supported KerML
   documents resolve against one coherent snapshot. Mixed-language source storage
   works; SysML documents explicitly report an unavailable frontend and affected
   lookup becomes Incomplete. This is not mixed-language semantic acceptance.
5. **Are imports/aliases/visibility handled by semantics rather than parsers?**
   Name denotation stays in KerML queries. Existing bounded declared/public lookup
   is retained. Complete imports, aliases, inherited lookup and access rules are
   not implemented; the parser does not decide their meaning.
6. **Does SysML semantics reuse KerML queries?** Required by ADR 0012 but not yet
   implemented. There is no `agq-sysml-semantics` crate or parallel semantic engine.
7. **Is Definition/Usage semantics centralized?** No semantic rules exist yet.
   Complete structural descriptors/views remain in `agq-sysml`.
8. **Are Part/Item/Attribute/Port/Connection semantics implemented according to
   authority?** No. Their exact retained rules, initial-slice classification and
   dependency requirements are inventoried. Checked structural views are not
   counted as language semantics.
9. **Are library-derived results explainable?** There are no library-derived
   conclusions yet. No parser builtins or fabricated library declarations are used.
10. **Are inherited features never copied?** Yes for the existing bounded KerML
    query slice and the new multi-document tests. A 49-document specialization
    chain returns the original feature and preserves its owner and membership.
    No SysML inheritance completion is claimed.
11. **Are incomplete semantics explicit?** Yes within the implemented boundary.
    Unavailable frontends, recovered namespaces, unsupported lookup and pending
    relationships remain explicit. Known members can be inspected without guessing
    a complete namespace. Syntax, resolution and KerML validation diagnostics are
    separate types/domains; no SysML validator or full validated-state contract exists.
12. **Are programmatic/textual models semantically equivalent?** Existing bounded
    KerML acceptance is retained. The requested SysML comparison, including
    library-derived conclusions, has not been implemented.
13. **Can all pinned standard-library files be processed without hidden syntax
    loss?** All 57 files are retained byte-for-byte and partitioned losslessly into
    UTF-8 token ranges. All have zero lexical-probe errors. All 36 KerML documents
    still require parser recovery; all 21 SysML documents lack a SysML frontend.
    The lexical inventory explicitly does not claim a complete production inventory.
    Canonical lowering, resolution and semantic validation are not run. The semantic
    quality command returns nonzero; unknown counts are null, never reported as zero.
14. **Are source identity and semantic identity separate?** Yes. Authored documents
    use independently allocated identities and immediate edit evidence. Document
    label changes and formatting preserve eligible IDs; replacement, deletion and
    cross-document moves allocate fresh IDs. Immutable library content uses its own
    provenance-qualified scheme and is not reconciled like mutable authored text.
15. **Are dependencies sufficient for future incremental computation?** Partially.
    Existing FactKey/search/proof contracts remain. Pending namespace populations
    participate in query context identity; a new document invalidates a previous
    root-namespace miss. Full-project rebuilding is currently the correctness
    strategy. Import/library/derived SysML invalidation tests remain unimplemented.
16. **Are all remaining semantic gaps in machine-readable coverage?** All metaclasses
    and their directly owned XMI rules have explicit status, and the 20 milestone
    gates have explicit outstanding work in `standards/v2-coverage.json`. Library
    reports expose every document's gaps. Complete grammar-production classification
    and additional specification-prose interpretation remain explicit unfinished
    obligations; the inventories must not be read as exhaustive semantic certification.

## Verification and limits

Commands, complete outputs and actual exit codes are in
`verification/sysml-semantic-foundation-v1/`. The final matrix checks formatting,
workspace Clippy/tests, generated descriptors, both structural runtime gates,
strict Rustdoc, standards, TypeScript/build, Node tests and browser tests. Strict
metamodel conformance and library semantic quality are separate failing gates;
normal regression success does not make them pass.

The archive loader reads only the pinned local content set, verifies every archive
and entry, checks metadata/dependency versions, traverses cycles with visited state
and retains junk/discrepancy evidence. It neither acquires nor rewrites artifacts.
The published AnalysisCases metadata mismatch remains a diagnostic. Apparent
rule/library naming discrepancies remain authority findings, not silent corrections.

Project publication is atomic in memory and uses a complete rebuild. It has no
incremental performance claim. Document lookup by ID is currently linear; lowering
and validation invoke bounded existing queries repeatedly. The 49-document test
is deterministic functional coverage, not a large-scale performance result.
Import-cycle, library-plus-authored, multi-overlay and deep-project stress acceptance
still needs the missing grammar and semantic foundations.

No persistence, API, diagrams, transformations, execution, simulation, application
migration or Pack 2B work is included. Generation-1 release obligations and the
completed generation-2 structural substrate remain separate from this incomplete
semantic milestone.
