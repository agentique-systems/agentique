# agq-history

History: a project's model folder on disk and its history in git
(ROADMAP §4.5, R-6; part `History` in `model/Agentique.sysml`).
It stores text and knows nothing about SysML; the System State
(`agq-system-state`, module `project`) prints the model and decides what an
element id and a locator are.

```text
<project>/                 a git repository: usually the project's code repository
└── model/
    ├── *.sysml            one file per document
    ├── agentique.json     {"format": 1, "next": <id>, "elements": {"<id>": "<locator>"}, "locks": ["<id>"]}
    └── .gitignore         keeps temporary and lock files out of git
```

- **Which repository**: the one whose working folder is the project folder.
  A project folder that is not the working folder of a repository becomes
  one (branch `main`), even inside another repository: a repository further
  up, such as one for the home folder, is never used.
- **Save** writes changed files to temporary files, makes a save journal
  (`agentique.pending`) durable, then moves the files into place and syncs
  the folders. After a crash the next open finishes or discards the save:
  old or new, never a mix.
- **One writer**: an open project holds `model/agentique.lock` locked; a
  second open fails with `Error::Locked`. The lock goes when the process ends.
  Opening waits up to half a second for a lock that looks held: on Linux a
  process another thread starts holds a copy of the lock file until it has
  started, so a lock released a moment ago can look held.
- **Reading versions** (`Revisions`): the model as committed and as saved,
  only read, with no lock and nothing written, so versions can be compared
  while the project is open for editing elsewhere.
- **Checkpoints** are commits of the model folder only; other work in the
  code repository, staged or not, is left alone. A model file edited outside
  Agentique while the project is open is neither overwritten nor committed
  (`Error::ChangedOnDisk`).
- **Branches** are the repository's branches. Switching is whole-repository
  and is refused while the model has changes since the last checkpoint.
- Refused with a plain message: unknown `format` or fields, and git conflict
  markers left by an unfinished merge.

Git is embedded through `git2` (vendored libgit2), so no git install is
needed. Merges are not done by git's text merge; merging by element identity
is left for later. The `crash-test` feature is for tests only.
