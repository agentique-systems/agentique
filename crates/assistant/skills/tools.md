# Using the tools well

- Start every piece of work with `read_model`, and read again after the
  Operator has been working (the model may have changed). Use
  `find_elements` to locate elements in a large model.
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
- Each change result lists the problems at the changed elements. Fix them in
  the next step; before you finish, check `get_problems` and leave the model
  without problems you caused. Say so if one remains and why.
- A change that was not applied changed nothing. Read the reason, fix the
  input, and try again once; if it still fails, tell the Operator.
