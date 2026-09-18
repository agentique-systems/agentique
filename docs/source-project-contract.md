# Generation-2 source project contract

`agq-kerml-text::SourceProject` extends the existing lowering pipeline. Each
publication owns an ordered collection of immutable document revisions and one
kernel snapshot with one root namespace. Documents are ordered by their exact
project-local path labels; paths never allocate DocumentId or ElementId.

Apply a batch against an explicit project revision. Parsing and complete-project
construction occur before publication. Invalid edits, resource limits, duplicate
paths, stale revisions and kernel failures leave the prior publication unchanged.
Syntax recovery and unresolved references may publish inspectable working models;
they do not pass `is_complete_slice`. History retains the original source and
semantic revisions. This is in-memory language infrastructure, not persistence.

Unchanged documents retain syntax/revision identities. Declaration, membership
and relationship continuity reuses ADR 0006's immediate-edit evidence. A document
label rename preserves identity; copying/moving a declaration to another document
does not. Whole-source replacement and delete/recreate allocate fresh identities.
No reconciliation consults historical tombstones or matches qualified names.

KerML documents use the existing bounded grammar. SysML documents are retained
with `FrontendUnavailable` until the SysML frontend is implemented. They are not
parsed as KerML. Their possible root declarations become a semantic context input,
so affected reference resolution returns `Incomplete` and cannot publish guessed
relationships. This establishes mixed source storage, not mixed semantic acceptance.
Library/project dependency ingestion and broader resolution remain subsequent work.

KerML rule set `/5` adds pending namespace populations to context identity and
propagates pending specialization evidence into effective-feature completeness.
Queries from a working model retain its pending assertions. Existing fact/search
proof contracts and strict metamodel diagnostics remain separate and unchanged.

Current rebuild is deliberately complete: all declaration contributions are rebuilt
and all references resolved against the same candidate graph. Publication compares
against the previous snapshot, preserving semantic identities and retirement history.
No incremental cache or sublinear build claim is made.
