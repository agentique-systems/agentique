# Publication resource workflow

The first new medium run was externally stopped by the disk-reserve watchdog
after 1,615.64 seconds. It did not establish a semantic rejection or finish an
uninterrupted run. Peak process-tree private memory was 5,235,523,584 bytes;
the last observed free disk was 1,043,197,952 bytes. The command, output digest,
exit 124 and exact limits are retained in `publication-commands.json`.

The retained frontier files occupied about 230 MiB. A subsequent Windows
observation reported an allocated page file of 23,403 MiB, current usage of
4,444 MiB and peak usage of 12,320 MiB. There is no earlier page-file allocation
measurement proving the cause of the disk drop. Host paging and concurrent
build pressure are plausible contributors, not established semantic faults.

Completed disposable Cargo target directories were cleaned with `cargo clean`;
the actual commands and exits are in `commands.json`. Source files, original
standard bytes, accepted caches and checkpoint/evidence files were retained.
All further corpus runs are serialized with Rust builds. The workflow may also
clean the main target's disposable development profile while retaining its
release executable. No user application, VM or editor process is terminated,
and no system page-file setting is changed.

`medium-resume-pin.json` independently pins the interrupted output, journal and
archive. Its invocation 3, ContextualBindings round 23 frontier is unfinished.
Resume must complete that computation, and a separate fresh uninterrupted run
must pass the exact medium equivalence gate. Neither resumption nor the medium
candidate establishes accepted Systems authority.

The first resume execution exited 101 after 162.547 seconds: restoration of the
completed invocation 0 reconstructed a declared handle that did not satisfy the
frontend's exact candidate-handle invariant. This is a checkpoint integration
failure, not a successful resumed gate. Its complete command and output digest
remain in `publication-commands.json`. The invariant must be preserved by the
repair; removing it or treating equal-looking inputs as interchangeable is not
an acceptable fix. Further use of retained checkpoints requires authenticating
their original semantics and proving the repaired handoff against focused and
medium uninterrupted/resumed equivalence tests.

The repaired reader authenticates the entire declared input, then adopts the
caller's original transaction handle before ordinary overlay validation. See
`resume-handoff.md` for the exact kernel and scheduler regressions, including
the unchanged exact query comparator. This changes no checkpoint encoding,
source pin, language rule or scheduler computation.

The repaired real medium resume exited 0 after 1,180.25 seconds, with peak
private memory 5,380,255,744 bytes and no watchdog stop. It restored four
invocations (three completed), skipped 60 completed rounds, continued the
unfinished ContextualBindings round 23, and durably committed the converged
round 33 state. All 695 references are Complete, kernel obligations are zero,
and all 14,791 producer pairs and 417,786 requirements are closed. The separate
`medium-resumed-reference.json` check records exact equality with the pinned
historical report's identity fields and certificate digests. A fresh independent
uninterrupted run and authenticated selected-evidence comparison are still
required; this successful resume alone does not authorize the full attempt.

Subsequently the fresh uninterrupted run exited 0 in 2,389.688 seconds, with peak
private memory 5,365,264,384 bytes and no watchdog stop. The exact medium gate
passed, including authenticated selected proof/search evidence; see `phase-f.md`
and `medium-equivalence.json`. Windows page-file expansion continued during this
run despite no competing Rust builds. Completed handoff-test artifacts were
cleaned, and after the run the verified executable was retained separately before
cleaning disposable Cargo release/Rustdoc artifacts. The original sources,
libraries, accepted caches, checkpoints and verification evidence were preserved.

That equivalence result authorized the single full candidate. Its limits
are 3,600 seconds, 6,656 MiB private memory, 1,024 MiB free disk, and a 600-second
stall timeout observing changed frontier, planned population or closed-pair
count. Resource limits are workflow stops and cannot relax semantic acceptance.

The single full attempt stopped on wall time after 3,601.109 seconds (exit 124),
with peak private memory 6,057,992,192 bytes and minimum free disk 2,929,954,816
bytes. There was no memory, disk or stall stop. Strict frontier 27 reached
Complete at 3,449.325 seconds; all 26,532 applicable producer pairs and 452,052
requirements were closed. Invocation 3's converged round 28 checkpoint was
durably committed before the stop. The final report, receipt, bindings and
accepted cache were not issued. The remaining final facade checks cannot be
inferred from the converged frontier or the earlier construction reference audit.
No further full run or resume was performed after the one-hour budget expired.
