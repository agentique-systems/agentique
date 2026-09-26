# Native response and view recovery changes

This change addresses the independent [pending revision review](pending-revision-state-review.md). It changes native presentation/worker routing, not language semantics or platform commit authority.

- Lifecycle reconciliation has a separate response kind and request identity. A disposable query or agent dismissal cannot fence a successful validation or the recovery of an unresolved commit. Model actions stay unavailable if the authoritative state query fails; the operator can refresh it explicitly.
- A durable commit acknowledgement retains the exact receipt independently of rendering. Project history is refreshed immediately with its own latest-request/project fence. A failed or superseded committed view retains a retry target; **Retry revision view** only loads that revision and refreshes history, without issuing another commit.
- Revision, comparison, candidate, candidate mode, saved presentation and agent return transitions synchronously stage the disposable scene before accepting the matching display. A rejected candidate view falls back to Current while retaining its semantic handle and lifecycle. Returning to an older display never restores an older candidate phase.
- Cancellation of a prepared candidate retains the handle until the platform acknowledges cancellation, so a failed cancellation remains recoverable.

The focused tests inject actual reply objects into a headless `StudioApp` created with eframe's test context. They exercise stale validation after agent dismissal, independent CommitUnresolved recovery, failed lifecycle refresh, commit followed by failed projection, project-scoped history fences, rejected revision/candidate/comparison scenes, and failed mode/agent-return transitions. These are native state tests with deliberately malformed fixture DTOs; they do not establish real semantic acceptance or runtime availability.

Recorded local checks: `native-state-format` and `native-state-format-check` both exited 0. Compilation, focused tests and Clippy are delegated to the integration lead's existing native target to avoid duplicate builds and disk use. Do not call those tests passed until the integrated run has completed.
