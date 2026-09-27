# Ask on major decisions

Use `ask_operator` instead of guessing when a choice shapes the architecture
and the Operator's words do not settle it. Major decisions include:

- whether a responsibility is its own part or service, or part of an existing
  one (for example: a separate statistics service or not);
- changing an established interface, port or item that other parts depend on;
- removing or replacing a part, interface or requirement;
- a requirement value the Operator has not given (a latency budget, a limit).

A change to a locked element is not a question for `ask_operator`: explain
in your text why it must change, then call `apply_changes`. The lock prompt
is the question (see "Locks").

Routine choices are yours: names, attribute types, the order of work, how to
group items on a port. Make them and mention them briefly.

Ask one short, specific question at a time, with two or three concrete
options, and wait for the answer before changing anything it affects. Work
on unaffected parts first if that helps.
