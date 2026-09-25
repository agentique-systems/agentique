# Accepted runtime asset search

Started from a clean worktree. `git fetch origin main` returned 0; both HEAD and
fetched main were `8b7f5ef79d51e6a3380f659bd1b32897cb6f6b2b`. Created
`platform/agentique-studio-first-light` from `origin/main` (exit 0).

No accepted publication bytes were located in the checkout, other directories
under `C:/Users/phili/github`, `.codex`, Downloads, Documents, Desktop, `.cache`,
`.local`, source or OneDrive. Filename searches covered canonical publication,
KerML/Systems cache and accepted bundle names. This is a bounded search, not proof
that no copy exists anywhere. No runtime store existed at `~/.agentique`; neither
legacy cache environment variable was configured. Git's historical worktree
entries point to directories that are now absent.

The all-ref Git object path search for canonical publication ZIPs, accepted
runtime packages and named KerML/Systems cache ZIPs returned no objects.
`.gitattributes` defines no LFS transport, and the repository's only CI workflow
uploads verification evidence rather than runtime publications. No additional
runtime artifact channel was found in the project documentation or tools.

GitHub's repository releases API returned `[]`. Its Actions API returned 80
artifacts, all named `verification-evidence` (largest 170,845,962 bytes). The
latest main run, `36098903068`, was downloaded with:

```text
gh run download 36098903068 --repo agentique-systems/agentique --name verification-evidence --dir verification/generated/first-light-ci-inspection
```

The command returned 0. Inspection found 2,950 files, zero publication cache
archives and one ZIP containing a historical browser trace. The automated
inventory command, API outputs, exit codes and output hash are retained in
`commands.json` / `asset-inventory.log`. No asset was published or reconstructed.

Historical evidence identifies the accepted KerML cache as 540,739,848 bytes
(`../kerml-v9-publication/summary.json`). The accepted Systems cache's recorded
SHA-256 is
`cfa48799aee1abf0e469c07380881b8d9a32c273bd3943713a47473808e8d5f7`
(`../final-audit-semantic-closure/accepted-artifact-gate.json`). These observations
help locate old bytes; they do not replace runtime receipt authentication.

The host initially had about 280 MB of disk free. Standard package-scoped Cargo
cleanup reclaimed regenerable artifacts: `cargo clean -p agq-studio` returned 0
(2,443 files / 3.5 GiB), and `cargo clean -p agq-kerml-text` returned 0
(6,435 files / 12.8 GiB). Original standards, library sources, databases and
publication receipts were not cleanup targets.

Browser skill setup succeeded, but selecting the local Studio URL returned
`No browser is available`. After reading its troubleshooting guidance, the
browser discovery list was `[]`. Repository Playwright provides local automated
browser verification; this does not supply the absent semantic runtime.
