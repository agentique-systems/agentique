# Startup feedback after actual first light

The first real run exposed raw worker enum names and several minutes without
useful visible phase context. This native-only follow-up replaces that text with
human-readable observations: locating the runtime, checking each language
library, opening the repository, validating system/agent architecture, and
restoring revisions. The current phase and its explanation are visible directly
on the opening card.

Elapsed time is measured locally from a successfully queued runtime opening.
The total survives phase changes; the phase timer starts when a different worker
phase arrives. Both stop at the matching terminal response. A new successful
open/install request starts a fresh timer. Stale request IDs and late phases
cannot reset or revive completed timing. The one-second refresh and optional
spinner are presentation only; reduced motion omits the spinner. There is no
percentage estimate, invented completed checklist, or inferred validation state.

The original setup copy recommended installing a runtime whenever there were no
projects, including after a model validation failure with a valid runtime. A
failed opening now states that the workspace could not open, retains its last
observed phase/time, and exposes the exact error in Opening details. Runtime
selection and the existing authentication/install path remain available.
The requirement label says which runtime is required rather than suggesting the
currently selected input has already been accepted.

The timer deliberately covers runtime opening/bootstrap only. Subsequent project
selection says that the selected revision is opening; its disposable query is
not reported as continuing runtime authentication. Internal closure phases that
the worker does not emit remain unclaimed. This cannot turn a Working model into
a Validated revision or alter any runtime authority/error/retry decision.

Two focused tests cover request-scoped timer completion/retry and the complete
existing serialized runtime-phase vocabulary. Formatting and diff checks were
run; compilation and actual updated startup/restart capture belong to integration.
The running first-light process and its retained evidence were not modified.
