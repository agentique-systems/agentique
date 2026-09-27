# agq-history

History: a project's model folder on disk and its history in git
(REALIGNMENT §3.5, R-6; part `History` in `models/agentique/Agentique.sysml`).
It stores text and knows nothing about SysML; the System State
(`agq-system-state`, module `project`) prints the model and decides what an
element id and a locator are.

```text
<project>/                 usually the project's code repository
└── model/
    ├── *.sysml            one file per document
    ├── agentique.json     {"format": 1, "elements": {"<id>": "<locator>"}, "locks": ["<id>"]}
    └── .gitignore         keeps temporary and lock files out of git
```

- **Save** writes changed files to temporary files, makes a save journal
  (`agentique.pending`) durable, then moves the files into place. After a
  crash the next open finishes or discards the save: old or new, never a mix.
- **One writer**: an open project holds `model/agentique.lock` locked; a
  second open fails with `Error::Locked`. The lock goes when the process ends.
- **Commits** (checkpoints) contain only the model folder; other work in the
  code repository is left alone. A folder outside any repository becomes one,
  on branch `main`.
- **Branches** are the repository's branches. Switching is whole-repository
  and is refused while the model has changes that are not committed.
- Refused with a plain message: unknown `format` or fields, git conflict
  markers left by an unfinished merge, and model files changed on disk
  outside Agentique while it is open.

Git is embedded through `git2` (vendored libgit2), so no git install is
needed. Merges are not done by git's text merge; merging by element identity
is left for later.
