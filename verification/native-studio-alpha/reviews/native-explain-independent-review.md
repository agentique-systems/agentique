# Explain summary: independent source review

Reviewed exact commit `12313c7147b6fc6a923290f94c1d54d08bf99e5f` against the
[real clipped Explain image](../real-run03/gallery/05-explain.png).
**Source judgment: pass, conditional on integrated tests and an actual new image.**

The summary now finds the selected subject's consequence, its actual incoming
rule, and only that rule's connected immediate support. An unrelated rule earlier
in the DTO does not become the cause by list order. Display arrows come from
actual explanation edges between retained nodes. The two- or three-support
summary cap leaves the exact explanation DTO and Evidence/Advanced contents
unchanged, and the UI states both its shown support count and partial-proof scope.

The readable Subsetting consequence label has appropriately narrow guards:
projection and explanation revision match; the selected proof is an Element fact;
exactly one projected edge has the original relationship ID; its revision, rule
and origin match the explanation; its exact kind/family are directed Subsetting;
both original endpoint nodes exist at the same revision. Property proofs,
ambiguous edges and stale or absent context fall back to the exact proof label.
The sole friendly rule title maps the exact known producer name to
`Suboccurrence specialization`; it changes no applicability or semantic result.

The layout limits the causal diagram to at most 268 pixels in the source tests
at widths 320, 599, 600 and 720 pixels, with no overlapping or out-of-bounds cards.
The support, rule and consequence therefore share a bounded area instead of
centering the conclusion below a long scrolling evidence column. Source tests
also cover an unconnected rule, unchanged exact DTO, ten malformed endpoint-label
contexts and a property-fact fallback.

No provenance or false-cause blocker was found. No build or runtime was invoked
by this reviewer. The integrated tests still need to run, and the actual real
proof must be recaptured to verify legibility, long names and visible conclusion
at the normal native window size. Geometry tests do not establish that product
judgment by themselves.
