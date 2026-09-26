# Distribution and additional-platform qualification

2026-09-26 read-only GitHub assessment; exact commands/outputs/exit codes are in
distribution-ci-assessment.json. No package was downloaded again, no runtime
consumer was launched, and no release or CI dispatch was performed in this work.

The repository is public and the current CLI identity has admin/maintain/push
permissions. Publication is technically available. The existing draft tag
runtime-kerml-v9-sysml-v3-bundle1 retains all four original assets. Its package
is 610,190,454 bytes and GitHub reports the expected SHA-256
37edf34cc0220ecdded8e0162f3fc1955ee2f3849e6dccf7ba373833ade3b026.
There is no reason to recreate or overwrite it. The integration lead holds
publication until the current facade authentication gate passes.

Latest main CI at 98c881f0bbac2c55e2777664633a50cb67ab7bf0:
https://github.com/agentique-systems/agentique/actions/runs/36227282323
The native-studio Linux job passed native formatting, clippy and tests. It is
headless build/test evidence, not interactive Linux qualification. The separate
checks job failed only independent-fixtures; its log reports exit 0 for workspace
Rust tests, clippy, npm check/build/test/e2e and the other recorded gates. This
historical main result does not validate this mission's new code.

Changes prepared:

- The existing runtime-asset workflow accepts either draft or published tags and
  preserves metadata. It checks the independently supplied outer digest, invokes
  ordinary verify/install in a new isolated store, checks exact installed hashes,
  and retains every command, exit code, stdout/stderr digest and elapsed time.
  It has no release-publication operation.
- tools/verify-runtime-distribution.py is the same qualification entry point for
  local operators and CI. Three small fail-closed tests passed: wrong transport
  never invokes the installer, failed facade verification never invokes install,
  and successful command completion cannot mask a changed installed manifest.
- Installation documentation now includes the exact outer hash comparison,
  source/offline prerequisites, explicit draft visibility, isolated qualification
  command and the intended workflow dispatch. Public release notes are prepared
  separately without claiming Alpha acceptance or current performance.
- A macos-14 native compile/link job is added alongside existing Linux tests,
  using the pinned independent lock and two build workers. It has not run yet;
  no macOS qualification is claimed. A mission-branch push will exercise both.

Final publication step for the integration lead, after successful authentication
and review of runtime-public-release-notes.md:

```powershell
gh release edit runtime-kerml-v9-sysml-v3-bundle1 --repo agentique-systems/agentique --draft=false --prerelease --latest=false --notes-file verification/native-studio-acceptance/runtime-public-release-notes.md
```

This publishes existing bytes and changes visibility; it is intentionally not
part of the build or qualification script. After publication, verify ordinary
unauthenticated release visibility and retain the release URL/metadata.
