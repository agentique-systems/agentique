# First actual native run: semantic baseline reached, visual journey failed

The actual native executable authenticated the installed accepted KerML v9 and
Systems v3 runtime, seeded all six current `models/agentique` sources through the
ordinary ModelingService and durable repository, and reached a Validated head.
The [journey report](journey.json) retains its complete manifest, exact source
digests, publication identities and validation receipt. The initial projection
contained 38 nodes and 37 relationships with Complete producer closure.

The run failed with exit 2 after 984.637 seconds; its peak process working set was
5,424,386,048 bytes. [The process record](process.json) identifies the actual
executable and command. These timings include a scenario scheduling stall and
are **not** a measured ordinary first-view latency. No real screenshot, candidate
or candidate commit was obtained. Both the repository and failed evidence remain
intact.

## Diagnosis and correction

After bootstrap the window remained responsive and visible but the scenario
stopped advancing when visible widgets became idle. A single diagnostic Win32
`RedrawWindow` request resumed its normal input path; it then passed the baseline
assertion. This request injected no semantic data or user command. Screenshot
delivery subsequently timed out. A later attempted heartbeat helper could not
attach because the process had already exited; its failure is retained separately.

Pinned `eframe 0.33.3` calls `raw_input_hook` before `egui::Context::run`.
Pinned `egui 0.33.3` clears non-outstanding delayed repaint requests at the start
of a pass. The real scenario requested its delayed input clock from the earlier
hook; fixture scenarios' immediate requests had masked this ordering defect.
The correction schedules the opt-in scenario's next repaint from `App::update`,
after the pass has begun. Ordinary application work still wakes the UI through
the worker's repaint callback. This change has no model or receipt authority.

The next gate must reopen the exact accepted baseline through normal runtime and
repository restoration, recheck the manifest and source identities, and execute
the entire visual/candidate journey. A report, manual redraw or successful unit
test cannot substitute for that gate.
