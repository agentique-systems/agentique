# Independent review: immediate owning Type scope

Production `56fde0d` independently reviewed: GO, no finding. The five new
controls passed (1.17 s), and the combined per-effect/immediate-owner control
set passed all ten tests (1.84 s). Library/test Clippy with warnings denied and
workspace formatting passed; every final command exited 0. Commands are recorded in
`owning-type-scope-independent-commands.json`.

The selector matches `owning_type`: Feature subject, its owning FeatureMembership,
then that membership's owning Type. Both canonical backing hops are checked.
It excludes lexical containers and non-FeatureMembership carriers. Tests cover
reparenting, detached carriers, truly absent canonical inverse navigation, and
NotComputed/Incomplete/Invalid inverse aliases. Unknown ownership, pending
namespace providers, Ownership effects and reference-scalar writers retain the
conservative fallback.

Future immediate-owner writes use exact possible attachment roots; custom
`SubjectAndOwners` contracts retain transitive ancestor reach. The independent
future-reader and certificate-mask assertions cover both. Semantic-effect and
fresh-attachment audits accept the subject/direct owner and reject ancestors
and unrelated targets. Canonical IDs, historical KerML publication bytes and
query evidence are unchanged.

This review ran no package sweep, standard-library cache load or corpus. Systems
publication and whole authored-fixture acceptance remain separate gates.
