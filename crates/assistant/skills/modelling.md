# Modelling in Agentique's SysML subset

The model is SysML v2 textual notation, limited to the subset below.
`read_model` shows it as text. Anything outside the subset is kept but not
checked, and it is reported as a problem; do not write it.

## Definitions and usages

- A definition is a reusable type: `part def`, `port def`, `item def`,
  `attribute def`, `interface def`, `connection def`, `requirement def`.
- A usage is a feature typed by a definition: `part store : LinkStore;`,
  `port links : LinkStorePort;`, `attribute clicks : ScalarValues::Natural;`.
- Model each component as a `part def`. Compose the system as one `part def`
  (for example `UrlShortenerService`) whose `part` usages are the components
  and whose `interface` and `connection` usages wire them together. Add one
  top-level `part` usage of it (`part shortener : UrlShortenerService;`) so
  requirements can be satisfied by its features.
- `:>` specialises a definition (`part def SqlLinkStore :> LinkStore`): a
  variant of the same concept. `:>>` redefines an inherited feature
  (`attribute :>> capacity = 1000000;`).
- Multiplicity: `[1]`, `[0..1]`, `[1..*]`, `[*]`. A part without one counts as
  required.

## Items and ports

- An `item def` is something exchanged: a request, a response, an event, a
  record. Give it attributes.
- A `port def` groups directed items, seen from the part that owns the port:
  `in item request : ShortenRequest; out item link : ShortLink;`.
- The other side of a connection uses the conjugated type `~PortDef`, which
  reverses every direction: `port storage : ~LinkStorePort;` on the client
  fits `port links : LinkStorePort;` on the store.

## Interfaces and connections

- An `interface def` has two port ends facing each other:
  `end port client : ~LinkStorePort; end port store : LinkStorePort;`.
- Inside the composing part def, wire parts with a typed interface,
  `interface storage : LinkStorage connect api.storage to store.links;`, or an
  untyped connection, `connection statsQuery connect api.stats to clickStats.stats;`.
- Ends are feature chains from the owner: `api.storage`. Connected ports must
  fit: every directed item meets a same-named item of the opposite direction
  with a compatible type.

## Attributes and values

- Attribute types come from the built-in `ScalarValues` package: `Boolean`,
  `String`, `Integer`, `Natural`, `Positive`, `Real`. Always write them
  qualified, `ScalarValues::String`: the tools cannot create imports, and a
  new package has none.
- Values are literals only: `= 50`, `= "text"`, `= true`, `= 1.5`.

## Requirements

- A `requirement def` states one checkable need. Its `doc` is the requirement
  text; its `subject` names what it constrains; attributes carry its
  parameters: `requirement def FastRedirect { doc /* A redirect is answered
  within the latency budget. */ subject api : HttpApi; attribute maxLatencyMs : ScalarValues::Natural; }`.
- A `requirement` usage applies it to this system, with values:
  `requirement fastRedirect : FastRedirect { attribute :>> maxLatencyMs = 50; }`.
- `satisfy fastRedirect by shortener.api;` names the feature that meets it; the
  feature's type must fit the subject.

## Not in the subset

Actions, states, calcs, constraints, use cases, flows, messages,
successions, bindings, allocations, actors and stakeholders, enums,
occurrences, `ref` usages, default values (`:=`), expressions, `ordered`,
short names and aliases, metadata, views. Behaviour comes in a later stage;
model structure, interfaces and requirements now. (Imports are part of the
subset and may appear in a model the Operator wrote, but the tools cannot
create them.)

## Reusing building blocks

The Library holds reusable definitions, shown as building blocks: the
project's own definitions, a small built-in library of neutral software
concepts (requests, responses, messages and events; request and message
ports; service, gateway, proxy, rate limiter, authenticator; store,
key-value store, cache; queue, topic, worker, scheduler; retry, circuit
breaker, fallback; and composites: cached store, rate-limited API,
asynchronous worker, event processing), and the Operator's My Library.

Before you model a common concept, ask in this order:

1. Does the project already define it? (`find_elements`, or `search_library`
   with scope "project")
2. Does a Library block mean the same thing? (`search_library`, then
   `read_library_block` to check its purpose, ports and parts)
3. Is a block close, with a difference that values or a specialisation
   express? Use it with `values`, or specialise it.
4. Is the concept really different? Then model it in the project.

Reuse only when the meaning fits. Never bend a concept to fit a block: a
database is not a `Queue`, and a block whose ports carry the wrong items is
not a fit. When you reuse, say which block and why it fits; when nothing
fits, say so briefly and model a plain project definition.

Four different changes; choose deliberately and say which you make:

- **Add a usage**: `use_library_block` adds a usage typed by the block. The
  block's inside stays in its definition; the usage shows its ports. Values
  given with it (`values`, such as `{"cache.ttlSeconds": 60}`) redefine
  inherited attributes in that usage only.
- **Override one feature in one usage**: redefine the inherited feature
  inside the usage (`attribute :>> ttlSeconds = 60;`; with `apply_changes`,
  a create in the usage with `redefines`). Only that usage changes.
- **Specialise**: a new definition in the project's own package that
  specialises the block (`part def SessionStore :> CachedStore`), with its
  redefinitions; type the usages that need the variant by it. The original
  is unchanged. Prefer this for a reusable variant.
- **Edit the definition**: changes every usage typed by it. Name those
  usages before you edit it. A definition in the project's `Library` package
  is a copy of a Library block; prefer a specialisation, and ask the
  Operator before changing the copy itself.

The project's `Library` package holds the copies of the blocks it uses; do
not put the project's own definitions in it. Save a definition to My Library
(`save_to_library`) only when the Operator asks for it; the Operator
confirms the save.

## Names

Definitions and packages in UpperCamelCase (`LinkStore`, `ShortenRequest`),
features in lowerCamelCase (`store`, `longUrl`). A name says plainly what the
thing is, in standard technical terms.

## Example

```sysml
package UrlShortener {
    item def ShortenRequest { attribute longUrl : ScalarValues::String; }
    item def ShortLink {
        attribute code : ScalarValues::String;
        attribute longUrl : ScalarValues::String;
    }
    port def LinkStorePort { in item save : ShortLink; out item found : ShortLink; }
    interface def LinkStorage {
        end port client : ~LinkStorePort;
        end port store : LinkStorePort;
    }
    part def HttpApi { port storage : ~LinkStorePort; }
    part def LinkStore { port links : LinkStorePort; }
    part def UrlShortenerService {
        part api : HttpApi;
        part store : LinkStore;
        interface storage : LinkStorage connect api.storage to store.links;
    }
    requirement def UniqueCodes {
        doc /* Every short code maps to exactly one long URL. */
        subject store : LinkStore;
    }
    part shortener : UrlShortenerService;
    requirement uniqueCodes : UniqueCodes;
    satisfy uniqueCodes by shortener.store;
}
```
