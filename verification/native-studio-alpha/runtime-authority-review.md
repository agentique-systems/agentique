# Independent accepted-runtime authority review

Date: 2026-09-25. Reviewer: experience subagent, independent of the runtime
implementation stream. This is a bounded source and focused-test review, not
evidence that rematerialization finished or that Native Studio loaded the real
Agentique model.

## Verdict

No acceptance bypass was found in the reviewed KerML transport recovery path.
It separates semantic publication authority from cache encoding and runtime
distribution. Promotion remains conditional on the ordinary Rust KerML and
SysML facade restores authenticating the result against the checked-in accepted
contracts. Neither a successful producer process nor this review establishes
runtime acceptance.

The user authorized deterministic reproduction of the already accepted inputs,
profiles, producer rules and bindings, not new standards, profiles, conformance
claims or replacement semantic receipts. The reviewed mechanism fits that scope
because it requires the original accepted payload identities again.

## Review boundary

Read the architecture and semantic-kernel guidance and the generation-2
coverage authority records, then inspected the current main-checkout versions of:

- `tools/restore-accepted-kerml-transport.py`
- `tools/runtime-recovery/kerml_rematerialize.rs`
- `docs/runtime-rematerialization.md`
- `docs/runtime-publication-distribution.md`
- `.github/workflows/runtime-asset.yml`
- `verification/native-studio-alpha/runtime/rematerialize_kerml.py`
- `verification/native-studio-alpha/runtime/rematerialize_systems.py`
- The retained producer identity and successful historical-build records.
- The existing KerML publication/cache restore, accepted overlay restore, SysML
  publication restore and runtime-publications packing/verification paths.

No language implementation, accepted receipt, binding manifest or native source
was changed by this review. Main was being developed concurrently; this is a
review of the inspected mechanisms, not an attestation of a frozen release tree.

## Why the transport operation preserves authority

The Python tool reads authority from the repository's existing accepted KerML
receipt. It permits generated-receipt differences only in the complete-overlay
snapshot revision label and its resulting graph length/hash. All other receipt
fields must compare equal before recovery proceeds.

The only payload edit is the single snapshot revision label. It does not replace
revision-shaped text elsewhere. It checks the generated input lengths and hashes,
then independently demands the exact original accepted uncompressed facade and
kernel entry lengths and SHA-256 identities. A changed canonical record, ID,
binding-related payload or closure payload cannot pass merely by receiving an old
label. Unexpected ZIP entries are rejected. Existing outputs are not overwritten;
promotion uses an exclusive partial output and a no-clobber filesystem operation.

The outer ZIP encoding can differ while its authenticated uncompressed contents
match the original cache contract. That is a transport distinction, not a new
semantic publication. The tool explicitly reports that ordinary facade
authentication is still required and that the authority receipt was not changed.

## Producer and facade checks

The wrapper runs the existing full KerML publisher after its original preparation
path. It checks the accepted Operational v9 profile, exact source content identity,
query rule set, publication semantic digest and generated binding equality. It
does not describe its candidate output as authenticated runtime. Removing
development preflights and post-publication authored benchmarks does not remove
the publisher's closure and capability gates.

The retained orchestration additionally checks the expected historical producer
commit, an empty tracked diff and equality of the copied wrapper source, and
records source and binary hashes. This matters because the wrapper's producer
commit string alone is descriptive rather than proof of its checkout.

The existing Rust restore paths validate the checked-in receipt, actual entry
bytes, graph identity, bindings and closure context. KerML restoration re-encodes
the restored overlay and verifies its accepted identity. SysML restoration checks
its exact mounted KerML dependency and its closure coverage as well as its own
publication identity. Runtime packing calls both facades before promoting the
bundle; runtime verification calls them again.

The Systems runner compares all accepted semantic receipt fields and bindings and
records whether the original transport entries match. It does not treat a
transport-entry mismatch as semantic acceptance: ordinary facade authentication
remains explicitly required. If that equality fails, the existing facade contract
must reject the candidate unless a separately justified transport-receipt path is
implemented and reviewed. This review does not authorize such a new path.

## Distribution review

The manually dispatched workflow downloads a named asset from a draft release,
requires an independently supplied SHA-256 of the outer bundle, rejects unexpected
bundle entries and runs the ordinary locked/offline Rust verifier. It has read-only
repository permission and retains verification evidence; it does not publish the
draft or activate semantic authority. Action references are pinned. The supplied
outer hash identifies transport; the Rust facades supply semantic authentication.

## Followups reported to the runtime owner

1. Align the older distribution document's
   `agentique-runtime-kerml-v9-sysml-v3.agq-runtime` example with the workflow's
   `accepted-runtime.agq-runtime`, or explicitly document the rename. Otherwise an
   operator following the older example cannot run the new download step.
2. Prefer the guarded provenance runner in reproduction instructions. A direct
   wrapper invocation should include the same checkout/source identity checks;
   its hardcoded producer commit field must not be treated as proof by itself.
3. Keep runtime acceptance pending until both ordinary facades and the packaged
   bundle verifier succeed. Then separately run real-model first light and the
   durable Native Studio journey. None of the fixture transport tests substitutes
   for those gates.

These are bounded operational/documentation followups, not grounds to rewrite the
accepted semantic receipts. The first two were sent directly to the runtime owner.
The runtime owner subsequently reported filename alignment in commit `61c841d`
and agreed to make the guarded runner the preferred reproduction path. That
followup does not change the requirement for actual facade authentication.

## Independent verification

Executed in `C:/Users/phili/github/agentique-systems/agentique`:

```text
python verification/scripts/test_runtime_rematerialization.py
.......
----------------------------------------------------------------------
Ran 7 tests in 0.054s

OK
exit code: 0
```

The tests cover exact payload restoration, rejection of semantic identity changes,
canonical-record mutation, forged input hash, replacement-authority ZIP entries,
output overwrite and accidental revision-label rewriting outside the snapshot.
They intentionally use miniature fixture payloads. They establish the transport
gate's rejection behavior; they do not establish that the multi-gigabyte accepted
cache was reproduced or authenticated. No duplicate Cargo build was launched.
