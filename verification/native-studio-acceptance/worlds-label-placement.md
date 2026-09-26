# Dense relationship and long-name follow-up

Automatic labels now avoid route segments through a screen-space cell index, as
well as canonical port markers, node cards and earlier labels. The index is built
only when labels are actually requested. Explicit selection retains a bounded
callout when no collision-free location exists; canonical routes remain unchanged.
Relationship labels wrap at two lines within a maximum of 35% of viewport width
(80–320 points). Full relationships stay available through Inspector.

Inspector titles wrap to three lines with ellipsis for extremely long names.
The full name remains in a tooltip and Copy full name context action. Feature and
relationship links wrap instead of widening the panel.

New tests exercise 20 parallel routes plus 20 crossing routes (automatic labels
must yield; explicit inspection stays available), and 1,000-character qualified
names at three inspector widths. Tests await root's integrated build; no fresh
visual or rendering-performance result is claimed. Format/diff checks passed as
recorded in worlds-label-checks.json.
