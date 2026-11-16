# Reopen (Continue) trace

## Flow
Home click -> bridge continue_work -> VoiceSocketClient.choose(work) -> daemon handle_trigger -> restore_from_facts -> OS restore.

## Work kinds table
| Work kind | Continue today | Result | Failure reason |
|---|---|---|---|
| Web page | choose -> daemon restore | often fails | OS open needs URL; scoped restore may not have URL |
| Browser chat | choose -> restore | fails | no tab restore path |
| File | choose -> restore | fails if moved | path stale |
| Folder/project | choose -> restore | fails if moved | path stale |
| App window | choose -> restore | fails | app not running |
| Empty work | choose -> restore | noisy | no target |

## Root causes
- continue_work silently returns outcome debug string for non-Restored.
- No status states surfaced to UI.
- No duplicate guard.

## Fixes needed
- Return structured status: opening/opened/partly_opened/couldnt_open.
- Add Activity entry each attempt.
- Duplicate guard by work id + short window.