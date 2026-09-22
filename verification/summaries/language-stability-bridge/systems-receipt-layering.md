# Systems receipt layering follow-up

This follow-up to held scaffold `c28fd16` removes SysML schema interpretation
from `agq-kerml-semantics`. It adds no crate, dependency, accepted publication or
catalogue entry. Restoration remains unavailable through the empty catalogue.

The private catalogue supplies the expected receipt-format label as data.
`TrustedPublicationReceipt` authenticates that envelope, the opaque binding
payload, named entry lengths/hashes and the exact producer context/certificate.
It exposes only immutable format and entry-name inspection. There is still no
public authority constructor, deserializer or implementable authority trait.

The SysML facade now owns its two schema labels, required receipt/binding
identity cross-checks, source-set agreement and exact three-entry population.
Those checks run before decoding the archive. Full standard binding reconstruction
and exact regenerated manifest comparison remain at the end of restoration.

Generic authority tests use a non-SysML format, an opaque binding object without
a format field, and a one-entry archive. They still reject a wrong compiled
format expectation and changed binding bytes. The facade regression separately
rejects every omitted or changed required receipt/binding field, including
matching omissions on both sides, and incorrect formats or source sets.

All four generic authority tests and four SysML restoration tests passed.
All-target Clippy for both changed packages, strict KerML semantic Rustdoc and
workspace formatting checks passed. No full publication or workspace test run
was repeated for this schema responsibility move.

`systems-receipt-layering.json` records focused commands and actual exits under
ADR 0021. This preparation does not replace the real accepted-cache restoration
and tamper matrix after Systems publication succeeds.
