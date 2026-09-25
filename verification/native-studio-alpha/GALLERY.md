# Native Studio product gallery

**Visual fixtures only. Real-model first light is pending accepted Systems runtime
authentication.** These are actual native-window captures, not image mockups or
HTTP scenes. Their adjacent JSON files identify the fixture, selected canonical
fixture IDs, displayed revisions and candidate state. Fixture
validation and commit remain unavailable.

The latest complete integrated gallery covers all eight product surfaces:

| Surface | Native capture |
| --- | --- |
| System World | [01-system-world.png](part-edit-after/01-system-world.png) |
| Focused subsystem | [02-focused-subsystem.png](part-edit-after/02-focused-subsystem.png) |
| Graph World | [03-graph-world.png](part-edit-after/03-graph-world.png) |
| Requirements World | [04-requirements-world.png](part-edit-after/04-requirements-world.png) |
| Explain | [05-explain.png](part-edit-after/05-explain.png) |
| History and difference | [06-history-diff.png](part-edit-after/06-history-diff.png) |
| Agent view | [07-agent-view.png](part-edit-after/07-agent-view.png) |
| Candidate | [08-candidate.png](part-edit-after/08-candidate.png) |

The [native input journey](part-edit-after/journey.json) passed. This gallery
includes the candidate intent/actor header, revision-bound edit dialog, bounded
Explorer labels and surface-recovery hooks. The
[command receipt](checks/part-edit-integrated-journey.json) retains the unchanged
executable SHA-256 before/after the run. Adjacent image sidecars include the PNG
digest. These visual captures do not exercise real RenamePart or semantic commit.

The [typography follow-up](reviews/typography-followup.md) retains the long-name
panel-width failure and its after capture rather than showing only short names.

Five independent review loops retain their before/after images, ranked
criticisms, changes and remaining weaknesses:

1. [Systems engineer](reviews/round-01-systems-engineer.md).
2. [Product designer](reviews/round-02-product-designer.md).
3. [Graph visualization expert](reviews/round-03-graph-expert.md).
4. [Game/editor interaction designer](reviews/round-04-editor-interaction.md).
5. [AI-native workflow designer](reviews/round-05-ai-workflow.md).

Additional reviews found revision/lifecycle recovery defects, source-command
identity risks and a runtime transport evidence race. Their fixes and actual
checks are retained in [reviews](reviews/). Completing those loops establishes
reviewed progress; it does not itself establish alpha product acceptance.
