# Retained real-journey bootstrap failure

The release executable in `../native-build-01.json` authenticated both installed
publications, then correctly left an explicit empty custom repository empty.
The real acceptance driver incorrectly expected the self-model to have been
imported there. It failed with `Authenticated bootstrap produced no project:
Runtime authenticated. Create your first project.` Exit code 2 is retained.

No project, candidate, validation, commit, restart or World acceptance is claimed
from this run. The fix explicitly authorizes self-model import only after the
fresh real-acceptance launch gate validates its isolated paths, using the ordinary
service/bootstrap journal. Ordinary empty custom repositories remain empty.

This run overlapped workspace tests. Its runtime timings and 5.44 GB peak working
set are diagnostic observations, not the isolated before/after comparison.
