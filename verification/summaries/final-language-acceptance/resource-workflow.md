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

The single full candidate remains gated on that equivalence result. Its limits
are 3,600 seconds, 6,656 MiB private memory, 1,024 MiB free disk, and a 600-second
stall timeout observing changed frontier, planned population or closed-pair
count. Resource limits are workflow stops and cannot relax semantic acceptance.
