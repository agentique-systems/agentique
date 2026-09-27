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
