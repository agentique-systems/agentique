# Runtime recovery evidence

This stream separates accepted semantic authority from cache recovery and release
transport. Original receipts, profiles and binding manifests are unchanged.

The final storage search found no accepted cache candidate. GitHub returned no
releases and zero Actions caches. The newest of 82 workflow artifacts was actually
downloaded: run `36150517543`, 146,311,299 bytes. It contained 3,046 files and only
one ZIP, the retained historical browser trace. Project-related local ZIP archives
contained no publication caches. `search-*.json`, `latest-ci-download.json`,
`latest-ci-inventory.json` and `local-archive-inventory.json` retain the observations.

The exact historical KerML producer at `a1a7b847` compiled successfully in 551.812
seconds. Its binary hashes are retained. The separate Systems producer/finalizer
uses `4ac9b8e`; its language source is identical to original closure `257d73cb`.
Recovery never passes `--write-bindings` or replaces accepted receipts.

At this evidence checkpoint, the real historical preflight/reconstruction chain
is running. A process start is not authentication. Completion records appear only
after commands exit, with actual exit codes and wall time; long semantic commands
also sample peak process memory. `rematerialize_kerml.py` and
`rematerialize_systems.py` encode the exact dependent sequence. The Systems stream
waits for strict original KerML payload reproduction before restoring its facade.

The new transport recovery utility allows only the historical random snapshot
revision label to be restored, then demands the original accepted payload hashes
and lengths exactly. Seven rejection/equivalence tests passed; see
`transport-recovery-tests.json` and its unmodified log. They establish the tool's
failure behavior, not acceptance of the missing real corpus.

The manual `runtime-asset` workflow authenticates an explicit draft asset against
its independently recorded transport SHA-256 and both language facades. It has
not been dispatched and does not publish releases. A downloadable runtime is not
claimed until a real pair passes and is packaged.
