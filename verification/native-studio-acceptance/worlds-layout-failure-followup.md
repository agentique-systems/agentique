# Integrated layout failures

The integrated native suite reported 139 pass / 6 fail. Two failures concern this
stream: the closed Diff toolbar consumed 127px versus its retained 110px ceiling,
and the qualified-name width assertion failed.

Diff's added Previous/Next buttons wrapped the owner row onto another line.
They now share the already-existing mode row; the owner row retains its original
purpose and the 110px test stays unchanged. The overview explicitly calls its
counts projected changes, not all semantic facts in the revision.

The name test incorrectly used a full-width CentralPanel followed by set_max_width.
egui Ui::set_max_width explicitly cannot shrink below the existing minimum; a
CentralPanel has already claimed the full available width. The test now creates
an actual exact-width SidePanel with zero margins. It retains the same three
width bounds and three-line height condition. Production also sets the layout
job's available width explicitly before handing it to the wrapping label.

Ran native cargo fmt and git diff --check, both exit 0 with no output. Integrated
rerun remains pending; no pass is claimed by this source-only diagnosis.
