# Typography and panel-width review

The native typography fixture uses the existing adversarial long-name scene. It
includes qualified names, Greek delta and mu, a technical pressure unit, and CJK
characters. These are explicitly fixture captures, not accepted model data.

1. **P1: Explorer contents widened the panel and squeezed Graph World.** The
   [before image](../typography-fixture/03-graph-world.png) showed a panel around
   570 pixels wide despite its configured maximum. The text widget's minimum
   content size overrode the intended panel constraint. Explorer rows now use a
   bounded selectable button with ellipsis, a full-name hover and their full
   accessible label. The [after image](../typography-width-after/03-graph-world.png)
   restores a 238-pixel Explorer and leaves the graph readable at 100% zoom.
2. **P2: Long engineering names need a complete reading surface.** The Inspector
   retains the full selected name and wraps it. The after image shows the Greek
   and CJK characters rendered with the installed fallback fonts. Canvas labels
   stay bounded; selecting an object exposes its complete identity in the
   Inspector. Very long names still consume substantial Inspector height.
3. **Qualification gap: mixed DPI and IME.** This is a normal-DPI native render,
   not a mixed-monitor or input-method qualification. No claim about composition
   input, spoken screen-reader output, or font availability on other operating
   systems follows from it.

Both complete eight-surface galleries and actual input journeys are retained.
The [after command receipt](../checks/typography-width-native-journey.json) records
exit 0, 23.921 seconds under simultaneous runtime-audit/build load. It is a
functional journey measurement, not a responsiveness benchmark.
