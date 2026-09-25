# Native Studio product language

Native Studio makes a system understandable before it exposes its metamodel. The
operator should see what belongs together, what connects, what is selected, and
what would change. Every visible engineering claim comes from the active
revision's projection; geometry and navigation are disposable presentation.

## Default hierarchy and language

The entry point is the project, then its systems, subsystems, components,
interfaces, requirements and behavior. Source files and exact metaclasses belong
in Source, Advanced and Explain. A part is called a Part, an interface an
Interface, and an unknown element remains an Element. Definition versus usage is
available in the Inspector; it must not dominate the architecture overview.
Engineering names are preserved exactly, including Unicode and technical units.
Long names are elided visually, never rewritten; hover and Inspector retain the
complete name. Counts describe the projection, never inferred verification.

## Worlds

System World is a spatial hierarchy: stable containment, quiet relationship
corridors, clear subsystem headings, and meaningful boundary ports. Graph World
is an investigation: relationship families, neighborhood expansion, selected
paths and precise edge inspection. Requirements World is a knowledge view:
subject, satisfaction and verification links are evidence of relationships, not
evidence of successful execution. History and Candidate are alternate revisions
of the same world, retaining identity, selection and the mental map.

Entering a subsystem preserves a breadcrumb and explicit Back, Forward, Up and
Home. Focus and fit may interpolate briefly; ordinary panning follows input
directly. Reduced motion removes interpolation. A candidate is always named
Working or Validated and visibly distinguished from the durable revision.

## Design system v1

| Treatment | Contract |
|---|---|
| Typography | 14 logical px body, 11 caption, 23 panel title; scene names use 16–17 world px with readable screen limits. Proportional UI font, exact names preserved. |
| Spacing | 8 px rhythm; 14 px scene inset; 30 px minimum chrome controls; 44 world px routing gutters. |
| Elevation | Dark canvas, quieter containment, raised component cards, highest contrast Inspector and transient labels. Light mode preserves the ordering. |
| Structure | Subsystem heading above contained parts; its boundary communicates membership. A restrained category mark and text identify the object. |
| Selection | Blue outline plus outer keyline; the Inspector uses the same semantic selection. Graph World brightens connected paths and fades unrelated edges. System World keeps child connections readable when their container is selected. |
| Hover / focus | Hover reveals exact identity and meaning; keyboard focus has an explicit stroke. Active controls have a filled treatment. Disabled commands retain reasons. |
| Authored / derived / standard | Authored relationships are solid, derived are dashed, and standard origin has an explicit text marker. Color is supplementary. |
| Requirements | Amber category treatment plus requirement shape/label. Link coverage is described literally, never pass/fail. |
| Behavior / agents | Violet category treatment plus distinct marks and names. Agent results are temporary overlays with target revision and authority. |
| Ports | Square boundary surface, filled center when a visible connection exists, hollow when no connection is shown. Names appear for the selected object, close inspection and small focused subsystems. Accessible names include the original owner and actual/unspecified direction. Placement never fabricates direction. |
| Candidate / diff | Working candidate uses an explicit lifecycle header, requested change and proposing actor; added uses + and green, changed uses ~ and amber, removed uses −, muted ghost and dashed relationships. |
| Motion | Short camera interpolation reinforces navigation. No semantic progress is simulated. Previous valid revision remains explorable. |
| Contrast | High contrast increases boundary and secondary text contrast, preserving labels and shape distinctions. |

The icon vocabulary uses simple vector marks in the spatial world: nested box
for system, square for part, boundary square for port, parallel lines for
interface, ruled page for requirement, rounded shape for behavior, diamond for
agent, dashed line for derived fact, and explicit Standard text for library
origin. No emoji or font-dependent glyph is needed to identify a scene object.

## Semantic zoom and density

At overview scale names remain only where a card has enough screen area to be
useful. At summary scale the hierarchy and names dominate. At feature scale
ports and concise part/interface counts appear. Near inspection adds exact port
names and selected relationship labels. Labels must not collide with card
titles or flood the entire graph. Selection may reveal detail one level earlier.

Collapse hides internal arrangement but retains external connection surfaces
using the original endpoint identities. A proxy is explicitly described as a
boundary representation, never a new semantic port. Expansion restores prior
geometry where possible. A local change should leave unchanged node positions
stable; median and p95 displacement are measured alongside overlap and routing.

## Review standard

A screenshot is evidence of rendering, not semantic acceptance. Fixture captures
are always labeled. Each review records the actual image, ranked criticism,
change, subsequent image and residual weakness. Public alpha acceptance requires
the operator journey, real accepted runtime first light when available, measured
responsiveness, and five independent product reviews. Features alone do not
earn acceptance.
