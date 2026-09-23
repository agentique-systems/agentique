# Actions residual population review

The `actions-exact-contributions` report records 532/534 Complete mandatory
references, zero kernel obligations, and incomplete producer closure. This is
not Systems publication acceptance.

The final diagnostics contain 106 closure, 11 value-context and 45
variable-featuring findings. Diagnostic-subject explanations show 105
`deriveUsageMayTimeVary` incomplete evaluations, 11 incomplete `FeatureValue`
evaluations, and two incomplete `PositionalRedefinition` evaluations. The latter
two are exactly TriggerAction's payload `70506936-f8f9-5057-9e36-37230a4e02df`
and receiver `a1ff5677-154c-54fa-b0b4-78c9a54a3c73`.

Syntax-only source mapping places the value-context findings in Actions, Flows,
Items and Parts. The final filtered causal trace has one direct writer:
TriggerAction's `deriveUsageMayTimeVary` blocks its `VariableFeaturing`.
The broad future-writer trace names
`042923d6-97a3-5b93-9d20-b4c1217ad9d9`, an Items `FeatureReferenceExpression`
for `edge` at bytes 2767..2771. That name is not a unique root-cause diagnosis:
the scheduler applies future-family effects once, using the first pending
creator as the trace label.

No independent semantic defect outside the reproduced positive-exclusion case
was established. The residual findings are consistent with a common pending
creator frontier, but the filtered trace and diagnostic-subject explanations
cannot prove that all 164 incomplete pairs are downstream of TriggerAction.
No additional semantic changes or corpus run were made for this review.

Independent review of `039a035` confirms that Action exclusion remains gated by
the composite flag; current owner, scalar and role evidence is retained; and
absence of a canonical positive witness retains the ordinary ancestry query
and negative typing-closure requirement.

Validation in the isolated closure checkout, with incremental compilation and
debug information disabled and one build job:

- `cargo test -p agq-sysml-semantics may_time_vary -- --nocapture`: exit 0,
  five tests passed, including positive Action/SelfLink/HappensLink exclusion
  and the existing negative-closure truth table.
- Existing `sysml_diagnostic_sources.exe` with a generated diagnostic-subject
  input: exit 0; only pinned syntax parsing and canonical source-ID mapping,
  without publication-cache restoration or producer replay.

Raw inputs and locator output remain under ignored
`verification/generated/residual-population/`. The earlier Actions report and
trace remain in the lead checkout's ignored verification directories.
