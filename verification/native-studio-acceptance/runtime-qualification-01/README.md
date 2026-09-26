# Public accepted-runtime distribution

Linux CI run [36231482800](https://github.com/agentique-systems/agentique/actions/runs/36231482800)
passed ordinary verification, installation into a fresh isolated runtime directory,
and status discovery at source `a6e1f41d2b549a8be10a79dfa3ec786c090b65b3`.
The qualification receipt binds the exact installer and all command outputs.
The integration lead rechecked every retained stdout/stderr digest after download.
The CLI used the workflow's release build without LTO; these lifecycle timings
are not comparable Native Studio performance measurements.

The existing [KerML v9 / SysML v3 bundle](https://github.com/agentique-systems/agentique/releases/tag/runtime-kerml-v9-sysml-v3-bundle1)
was then published as a prerelease, without replacing any of its four assets.
The original package remains 610,190,454 bytes with SHA-256
`37edf34cc0220ecdded8e0162f3fc1955ee2f3849e6dccf7ba373833ade3b026`.
Before/after asset identities and unauthenticated public visibility are retained.

The publication command, actual output and exit code are recorded in
`../../native-studio-alpha/checks/acceptance-runtime-publication.json` and its
matching `.txt`. This publishes the accepted language runtime, not an Alpha
acceptance claim for Studio. No accepted semantic receipt, normative library
content or runtime package bytes changed.
