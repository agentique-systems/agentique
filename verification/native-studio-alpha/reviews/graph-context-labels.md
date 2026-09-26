# Selected graph relationships at fitted zoom

The real run05 Graph capture fits ModelRepository's six-node neighborhood at
57%. All five incident edges are unlabelled because their labels previously
required Features LOD. The retained projection identifies two `typed by`, two
`owns`, and one `specializes` relationship; identical highlight color alone
does not communicate that distinction.

The renderer now permits selected-incident labels at Summary LOD in Graph World
when at most 24 edges are visible, with at most eight automatic labels. It keeps
the existing exact semantic label and derived marker. Explicit edge selection
and hover take priority. Dense unselected graphs do not gain all-edge labels.

The small relationship_labels module places screen-space annotations against
visible route segments, cards, container headers and higher-priority labels.
Attempts are bounded and deterministic. Automatic labels yield when no clear
placement is available. Explicit inspection retains a bounded fallback callout;
the relationship Inspector and hover details still expose the exact edge.
Unarrowed leaders point to an actual visible point of that edge's route.
No canonical record, endpoint, route, projection or relationship filter changes.

Two geometry regressions cover a globally long offscreen route, occupied card
space, competing labels, crowded explicit fallback and entirely offscreen
routes. Native execution and a new real screenshot remain required; source
implementation alone is not visual acceptance.
