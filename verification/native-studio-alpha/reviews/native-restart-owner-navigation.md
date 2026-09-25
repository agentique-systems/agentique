# Restart acceptance navigates to the committed part

Source-only fix against `7d31b98`. The fresh `--no-restore` System overview has
one architecture level. The created part belongs inside ModelingPlatform, so
requiring it in the opening projection prevented the restart journey from
reaching ordinary navigation. This was a scope error in the acceptance driver,
not evidence that the durable part was missing.

The restart plan now performs these actual input-driven stages:

1. Open the same project through its normal project button. Require the complete
   actual Validated manifest to equal the first process's committed manifest,
   including its source identities, checkpoint and validation receipt. Require
   the original project/branch and branch head.
2. Select the visible ModelingPlatform definition in Explorer and press F.
   Require the normal focused architecture response and actual repository context.
3. Select the nested part in Explorer. Repeat the durable manifest/head check;
   require the retained exact part and owner IDs, authored source-backed part,
   exact owner focus, and Inspector element/owner/revision. The existing selected
   object check also requires actual selection and matching Inspector binding.
4. Open History and repeat its unchanged durable revision check.

The driver creates no projection, scene or semantic state. It adds no model
wrapper handling or project defaults. The first-process step plan, all launch,
report-digest, source-manifest and resume guards are byte-identical to the base;
their source hashes and exact formatting/check commands are retained in
[native-restart-source-check.json](../checks/native-restart-source-check.json).

Two new unit regressions cover the actual restart step ordering and identity-gate
counterexamples: shallow/missing part, wrong focus, substituted ID, changed
canonical owner, stale or mismatched Inspector, derived replacement and wrong
World projection. These DTO counterexamples are not real semantic acceptance.
No Cargo build, test execution or runtime consumer was started by this change.
The integration lead must run native unit checks and the separate real restart
process against a successful first-process report before claiming durability.
