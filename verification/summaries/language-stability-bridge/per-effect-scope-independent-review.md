# Independent review: per-effect producer scopes

Reviewed production `7ff867f` independently. Scoped result: GO, no production
finding. Five adversarial tests passed (0.77 s); package library/test Clippy
with warnings denied and workspace formatting also passed. Controls cover:

- Narrowing/widening scopes, including an effective `Model` override on a
  default `Subject` descriptor and retained conservative non-typing masks.
- Future subject activation with `Subject`, `SubjectAndOwners`, and `Model`
  effects; absent current instances cannot erase future effects.
- Registry identity changes, rejection of stale direct/certificate transport,
  and rejection of overrides for effects the descriptor never declared.
- Exact semantic relationship, scalar, and existing-child ownership auditing.
- A local incoming relationship reader of a protected dependency: a mixed
  reference-scalar capability still invalidates it despite a narrow Featuring
  override. Protected ownership collections remain immutable.

Source review confirms effect-specific routing in direct/future causal readers,
closure masks and semantic/scalar/ownership audits. Primitive/reference scalar
guards, unknown selected populations, provider ownership and immutable boundary
handling remain conservative. Fresh attachment auditing deliberately retains the
default creation envelope. The new map is included in registry schema `/6`;
canonical output keys and historical accepted KerML bytes are unchanged.

The first boundary fixture expected an evaluated producer on a protected subject;
such producers are correctly inapplicable. The next fixture read the protected
owned collection, which is correctly immutable even with external references.
The final control queries incoming FeatureTyping sources from a local reader,
exercising the intended open external-carrier boundary. These setup corrections
and all actual command results are retained in
`per-effect-scope-independent-commands.json`.

No package sweep, accepted cache load, or corpus run was performed for this
review. The separately reported compound fixture still has an incomplete pair;
this scoped GO does not establish publication or readiness acceptance.
