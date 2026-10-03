# Using the tools well

- Start every piece of work with `read_model`: without an element it gives
  an outline (every element with its kind and type, locks and problem
  counts). Read an element by qualified name for its full text before you
  change it, and read again after the Operator has been working (the model
  may have changed). Use `find_elements` to locate elements in a large model.
- A long tool result is cut with a note; narrow the request as it says.
- Make one coherent `apply_changes` call per step, with a plain
  `description` of what the step does ("Add the link store and connect it to
  the API"); the Operator sees it in the history and can undo the step as a
  whole. Later operations in the same call can refer to elements created by
  earlier ones.
- Refer to elements by qualified name (`UrlShortener::LinkStore::links`).
  Types may use the name visible from where they are written (`LinkStore`).
- Operation examples:
  - part def: `{"op": "create", "parent": "UrlShortener", "kind": "part def", "name": "LinkStore", "doc": "Persists short links."}`
  - port with a conjugated type: `{"op": "create", "parent": "UrlShortener::HttpApi", "kind": "port", "name": "storage", "type": "~LinkStorePort"}`
  - directed item on a port def: `{"op": "create", "parent": "UrlShortener::LinkStorePort", "kind": "item", "name": "save", "direction": "in", "type": "ShortLink"}`
  - interface def end: `{"op": "create", "parent": "UrlShortener::LinkStorage", "kind": "port", "name": "client", "type": "~LinkStorePort", "end": true}`
  - wiring: `{"op": "connect", "parent": "UrlShortener::UrlShortenerService", "kind": "interface", "name": "storage", "definition": "LinkStorage", "from": "api.storage", "to": "store.links"}`
  - subject: `{"op": "create", "parent": "UrlShortener::UniqueCodes", "kind": "subject", "name": "store", "type": "LinkStore"}`
  - satisfy: `{"op": "create", "parent": "UrlShortener", "kind": "satisfy", "requirement": "uniqueCodes", "by": "shortener.store"}`
- The Library of building blocks: `search_library` finds blocks by words,
  kind, scope or fit with a port (`fits_port`); `read_library_block`
  describes one in words (read it before you use it); `use_library_block`
  uses one in a single change: it copies what the block needs into the
  project's `Library` package and adds a usage, with optional `values` and a
  connection (`connect_to`, a port as a feature chain from `parent`).
  Example: `{"block": "built-in:Library::Storage::Cache", "parent": "Shop::System", "name": "sessions", "values": {"ttlSeconds": 60}, "connect_to": "front.backend"}`.
  A reported conflict changed nothing: ask the Operator, then pass
  `if_exists`. `save_to_library` saves to My Library, only when the Operator
  asked.
- Each change result lists the problems at the changed elements. Fix them in
  the next step; before you finish, check `get_problems` and leave the model
  without problems you caused. Say so if one remains and why.
- A change that was not applied changed nothing. Read the reason, fix the
  input, and try again once; if it still fails, tell the Operator.
- Operating the application (C-53): `observe_app` tells you, as text, what
  Agentique shows now: its identity, the screen and `screenRevision`, the
  dialog, selection, status, the controls on screen (id, role, label,
  value, enabled) and the commands available. You never see pixels; never
  claim you saw the screen. `act_in_app` does one thing the Operator could
  do, visibly: a command by id, a click or fill on a control by id or label,
  keys, typing, selecting an element, opening a project, or waiting for a
  condition. Pass `expect.instance` and `observed` (the `screenRevision`)
  from your latest observation and say `why`; a stale action is refused and
  nothing happens: observe again. Prefer `apply_changes` for changes to the
  model; use the application to check what the Operator would see, to
  exercise a feature, or when the Operator asks you to show something.
