# Candidate review gate follows explicit operator selection

Run05 failed at `review Candidate revision`, but did not lose selection.
The actual before/after records retain the same ViewService selection
(`1ce77981-59b7-5a69-b978-88f84b7d1f59`), focus, candidate revision, Inspector and
camera. The background-read test had deliberately selected ViewService while
preparation ran. The driver then required the new part to be selected before its
next explicit `select real candidate part` step. Its failure message incorrectly
described preserved selection as lost selection.

[The source/evidence receipt](../checks/candidate-review-selection-source-check.json)
pins the exact failed report and matching before/after fields. Failed run05 is
retained as failed; this correction does not establish later acceptance or permit
resuming a process that attempted candidate preparation.

The driver now records the actual typed selection (all targets and primary) and
focus before each mode input. Initial Candidate review must preserve that exact
selection and candidate revision. It may not silently clear it or substitute the
created part. Only after the explicit part selection passes its normal Inspector,
canonical ID and source-backed owner checks does the driver remember that intent.
Subsequent Current/Candidate/Diff review requires the same added canonical part
to return when present, forbids its appearance in Current, and requires focus
continuity. Candidate identity, exact revision, matching before/after lens and
added-object diff assertions remain mandatory.

Independent source review caught a necessary detail in the initial correction:
raw selected-target identity does not prove that its geometric target exists.
The final later-review check retains `app.selected_element() == Some(added)`,
which resolves the target in the exact active scene. Current also rejects the
created ID anywhere in multi-selection, including a secondary target. Regression
counterexamples cover a Part ID disguised as an absent port, an absent node
target and a leaked secondary selection.

No production selection behavior changes. Forcing an automatic new-part selection
would override the operator's valid background inquiry merely to satisfy a faulty
test. The first-process and restart step lists, launch/report/source/resume guards
and restart identity proof are unchanged, with source hashes in the receipt.

Source review of the remaining journey found no second comparable anticipatory
selection assertion: candidate existence/owner/source proof follows preparation;
added selection follows its explicit input; lifecycle and durable manifest checks
follow validation/commit; restart checks the manifest before normal owner
navigation and checks nested identity afterward. These later steps still require
actual execution; source review does not predict they will pass.

New regressions exercise production StudioApp mode transitions using a clearly
labeled fixture: preserve an existing selection, select the created fixture part,
hide it in Current, restore it in Candidate and retain it in Diff. Gate
counterexamples reject lost/substituted selections, changed primary, stale
revision, changed focus and invalid explicit-selection evidence. Fixture state
tests do not establish semantic reconstruction or durable acceptance.

Formatting and diff checks passed. No Cargo build, test execution or accepted
runtime consumer was started by this change. The integration lead must run the
new regressions and repeat the real journey in an admissible repository state.
