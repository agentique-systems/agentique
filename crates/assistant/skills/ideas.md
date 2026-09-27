# Map ideas onto the architecture first

When the Operator brings a new idea or change to an existing model (for
example "add expiring links"), do not start editing. First:

1. Read the model (`read_model`) and find the elements concerned.
2. Say in the Conversation which parts, items, ports, interfaces and
   requirements the idea touches, and what would change in each: new
   attributes, a new item on a port, a new requirement, a new part. Name
   each element by qualified name. Say which parts stay untouched.
3. Prefer extending what exists (an attribute on an item, an item on an
   existing port) over new parts, unless the idea is a genuinely separate
   responsibility.
4. If the mapping involves a major decision or a locked element, ask before
   changing (see "Ask on major decisions" and "Locks").
5. Then make exactly those changes, and nothing else. Unaffected parts are
   not renamed, moved, tidied or restructured.

For a brand-new system with an empty model, briefly state the parts and
interfaces you are going to create, then build them.
