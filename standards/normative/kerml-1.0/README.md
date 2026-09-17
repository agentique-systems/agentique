# KerML 1.0 normative metamodel inputs

The original OMG bytes are preserved here. [lock.json](lock.json) records exact
hashes, authority, roles and retrieval metadata for all three inputs.

| File | SHA-256 |
|---|---|
| KerML.xmi | `45b18775afe2b2fcdc70e24f37c6d2f344defcc3f38a02075a193354e2d7b466` |
| KerML.json | `e454fe4b7c04f3d95874b6c1a4e6ef056ea5874c71fd3d17e3180319c2f58ab2` |
| PrimitiveTypes.xmi | `62d12217fcd26037fc917709e2a896600af574efd6412c110e2a711395a69849` |

KerML XMI is the primary metamodel source. KerML JSON is its normative serialization
schema and only a cross-check source for overlapping information. UML primitive
XMI supplies the external type declarations. Project/API schemas are not inputs.
See [ADR 0002](../../../docs/adr/0002-normative-metamodel-pipeline.md) for authority,
representation differences, importer scope and the identity policy.

`npm run standards:check` verifies these and all previously pinned standards.
`node tools/pin-kerml.mjs` explicitly reproduces missing files from the reviewed
hash lock, refusing changed upstream bytes or local replacements. No build or test
invokes that acquisition command. Retained original notices and the supplied OMG
publications govern the normative material; generated IR is not a new standard.
