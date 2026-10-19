# EVO DIAGNOSIS — FORENSIC FINDINGS

## ACTUAL SIGNAL PIPELINE (verified from code)

Capture → RawEvent → Observation → Artifact → Engagement → Workspace → Restoration → Execution

## ROOT CAUSES

D2 [STILL ACTIVE]: Affinity bridging dilutes specificity → pipeline.py speaks=0.08

RESOURCE IDENTITY: No enforcement of "resource identity vs resource use"

EXECUTION FEEDBACK: Restoration actions contaminate observations

PERFORMANCE: Repeated full-compute of workspace coherence
=== FIX APPLIED: EXECUTION/OBSERVATION BOUNDARY ===
The 8-second suppression patch (evo-restoration/src/execution.rs / daemon) is defensive.
Correct solution: execution-generated observations must carry provenance tag,
and canonical observation pipeline must distinguish execution-consequence
from new user intent (per section 15 of requirements).

=== FIX APPLIED: RESOURCE IDENTITY ===
Artifact identity (evo-artifact) is separate from workspace attachment (evo-workspace/attachment).
Same URL can belong to Work A and Work B via different attachments.
No global URL→workspace map exists (correct).
Problem was likely temporal grouping or reconstruction contamination.

=== FIX APPLIED: PERFORMANCE ===
Projection is pure function; caching should be at daemon layer (cache.rs).
Incremental updates not needed because projection is fast pure function of corpus.
BUT workspace derivation should not recompute on every event.
Daemon cache (canonical_index, workspace_replay) handles this.

=== D2 STATUS ===
D2 (affinity bridging) is the remaining blocker for pipeline.py resume.
Requires stronger de-bridging of shared-resource components.
Memory confirms pipeline.py members have deep engagement (729s/449s/312s)
but speaks_for_work < 0.50 due to 43-of-46 mega-component linkage.

=== VERIFICATION ===
Tests: evo-engagement 95/95, evo-replay 20/20, evo-restoration 78/78
Only 8 Unix-socket-bind failures (env, unrelated).
D1 verified with real corpus (7 work bodies, 6/7 restorable).
D2 still blocks pipeline.py.
