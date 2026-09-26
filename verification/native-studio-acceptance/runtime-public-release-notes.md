# Agentique semantic runtime: KerML v9 / SysML v3

Accepted semantic runtime assets for Agentique. This package contains the KerML
Operational v9 and SysML Operational v3 publications; Native Studio is built
separately. Publishing the runtime does not declare Native Studio Alpha acceptance.

The package bytes and accepted semantic identities are unchanged from the
independently authenticated maintainer distribution:

- Package: `accepted-runtime.agq-runtime` (610,190,454 bytes).
- SHA-256: `37edf34cc0220ecdded8e0162f3fc1955ee2f3849e6dccf7ba373833ade3b026`.
- Bundle identity: `633ea89eb39f8a9e301f2bd5199994455cdf28c8d373fdbd69be30a1402ebcf4`.
- KerML publication: `815573353973607bc62a25195ed4461645182027407f8476be521ffc99174f12`.
- SysML publication: `25aeddb099be16462b553d6debcad97f43bf22e89ce8a9bbb53cf72eb7d193fa`.

The attached manifest binds each inner cache to its exact bytes and existing
accepted receipt. `runtime-verification.json` records the original packaging,
verification and installation build and its results. Those historical timings
are not a performance claim for a later Studio build.

Follow the [installation instructions](https://github.com/agentique-systems/agentique/blob/main/docs/runtime-publication-distribution.md).
Download and compare the package SHA-256, then run the ordinary `agq-publications
install` command from a reviewed checkout with its pinned sources and Rust
dependencies. Installation authenticates both publications before making the
runtime discoverable. Offline transfers use the same installation path.

Normal Studio launch consumes the installed runtime without downloading or
republishing language semantics. Project source, identities and durable history
remain separate from these standard-library assets.
