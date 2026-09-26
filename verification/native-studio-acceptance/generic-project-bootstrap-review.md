# Generic project restart review

`open_authenticated` previously called `seed_agentique` for every selected database. The seed helper creates an Agentique project whenever that name is absent, so restarting a newly created generic project would add the sample beside the user's project.

Native bootstrap now starts that sample only for an empty default runtime repository. Existing Agentique projects still enter the original seed helper so a persisted pending seed plan, including a commit whose acknowledgement was interrupted, remains resumable. That helper retains its original exact metadata, source, branch and journal checks before changing an existing project. Other repositories do not even open a sample bootstrap journal.

The new unit regression covers first-run default storage, explicit empty storage, one or several generic projects at both locations, and an existing Agentique project beside a generic project. It requires no runtime authentication. It deliberately does not treat a project's name as semantic or validation authority.

Checks in the isolated worlds worktree:

* `cargo fmt --all` — exit 0, no output.
* `git diff --check` — exit 0, no output.

Compilation and the focused test remain with the integration lead's shared target. The main integration lead separately confirmed the previous compact Diff toolbar correction passes its unchanged <=110 px test.
