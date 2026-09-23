# Working-state acceptance preparation

Reviewed the strict accepted frontend at `695e67e`. `SourceProject::apply` still
rejects recovered production inputs, and `lower_accepted_source` still requires
strict promotion and supplies empty pending-scope sets. The additive Working
frontend and workspace remain necessary; neither is implemented by these tests.

Three held tests in `crates/modeling-workspace/tests/working_states.rs` require:

- targeted recovery/repair preserves a disjoint declaration's reconciled syntax
  and authored identity, while later explicit deletion/re-addition retires it;
- a failed mandatory reference retains its current document/source origin and
  incomplete effective query evidence, with no stale old endpoint;
- Parsed but unsupported variation remains Incomplete and cannot validate even
  when the actual producer status and whole-graph certificate are Complete.

The separate two-test `workspace_working_inputs` preflight passes against the
real Operational v2 parser (0.05 seconds). It confirms complete and malformed
source classifications, exact source roundtrip, disjoint syntax identity across
recovery and repair, and fresh identity for a new document. It does not establish
canonical lowering, workspace acceptance or publication success.

Targeted parser Clippy and both workspace/held-file formatting pass. The requested
held integration command exits 101 because its crate manifest does not exist;
the three semantic tests remain uncompiled and unexecuted. No manifest, Cargo
member, production code, accepted cache load or publication run was added.
`working-states-commands.json` records commands, exits, hashes and source identity.
