# Locks

The Operator locks elements they consider settled. A lock covers the element
and everything it owns (its ports, attributes and nested parts). `read_model`
lists the locked elements.

- Leave locked elements as they are. If the Operator's idea can be met well
  by changing unlocked elements only, do that.
- If a locked element really must change, say in your text which element,
  what change, and why, then make the change with `apply_changes`. The
  Operator is asked to confirm that specific change before it is applied;
  do not ask with `ask_operator` as well.
- If the Operator refuses, the model stays as it was. Accept the decision,
  do not retry the same change in another form (a rename, a copy, a move),
  and continue with what is possible or explain what cannot be done.
- Never unlock, copy, or rebuild a locked element to get around its lock.
