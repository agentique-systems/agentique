# Simple, general, plainly named architecture

Agentique exists to prevent slop. In architecture, slop is:

1. **Unnecessary complexity.** Use the fewest parts, ports and interfaces that
   carry the requirement. Do not add layers, managers, gateways, caches or
   queues nobody asked for. A small system stays small.
2. **Shallow concepts.** Before adding a part, look for an existing concept it
   belongs to. Two parts that do nearly the same thing are one part def with
   attributes, or one general part def with specialisations (`:>`). Never
   create near-duplicates such as `LinkStore` and `UrlStorage`.
3. **Invented naming.** Use standard, plain technical terms: `HttpApi`,
   `LinkStore`, `ClickStats`, `ShortenRequest`. No creative, branded or vague
   names (`LinkForge`, `Nexus`, `Manager`, `Handler2`). Model concepts keep
   their SysML names.
4. **Literal compliance.** Follow what the Operator means, not only the words.
   If a request would produce a poor structure, say so briefly and propose
   the better one.
5. **Drift.** A new idea must not reshape established, working parts. Change
   only what the idea needs (see "Map ideas onto the architecture first").
6. **Hidden architecture.** Everything that matters is in the model where the
   Operator can see it: components, what they exchange and why (short `doc`
   comments on part defs and requirements).

Aim for a stable core with room at the edges: well-defined interfaces between
parts, so a part can be replaced without touching its neighbours.
