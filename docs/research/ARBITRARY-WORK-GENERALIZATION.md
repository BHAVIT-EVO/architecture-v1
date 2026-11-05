# Generalizing work understanding without guessing

Updated 2026-10-06. This is an engineering decision record, not a claim that
Evo understands every person's work.

## Research conclusion

There is no credible single classifier that can infer arbitrary work reliably
from a desktop stream. The robust pattern is a layered system:

1. collect sparse, permissioned evidence with provenance;
2. keep task hypotheses separate from facts and preserve alternatives;
3. use temporal structure and resource relationships rather than app or
   profession labels;
4. use local semantic retrieval only to propose candidates, never to authorize
   a restore;
5. ask for correction when evidence cannot distinguish hypotheses; and
6. evaluate on held-out people, tasks, languages, and renamed resources.

This agrees with the W3C PROV-DM model's separation of entities, activities,
agents, derivations, and time, and with the 2026 FOCAL desktop-logging paper's
filter → plan → log → task-isolated-memory architecture. FOCAL reports better
interruption handling when task context is isolated, but its benchmark is not
evidence that Evo's thresholds generalize to real users.

## Changes made in Evo

### Language and retrieval

The old default remains English all-MiniLM-L6-v2. Evo now has an explicit local
`multilingual-e5-small` profile. The semantic interface supports document/query
roles, so E5 receives its required `passage:` and `query:` instructions without
breaking deterministic fixture embedders. Profile, cache, asset paths,
dimensions, and missing-model behavior are explicit; no profile downloads or
falls back to another model. Because MiniLM's similarity thresholds are not
calibrated for E5, a voice match is a suggestion requiring explicit `CHOOSE`;
fuzzy `SCOPE` refuses and an exact selected locator remains available.

Use only with separately provisioned local assets:

```sh
EVO_EMBEDDING_PROFILE=multilingual-e5-small \
EVO_EMBEDDING_MODEL_DIR=/absolute/path/to/e5 \
cargo run -p evo-daemon --offline --locked
```

The user explicitly authorized one public E5 download. All five assets at
immutable revision `614241f622f53c4eeff9890bdc4f31cfecc418b3` passed
size/SHA-256 verification outside Git. The production Rust loader passed a
real-weight offline smoke: four synthetic English, Arabic, Japanese, and Hindi
queries ranked a relevant bicycle-repair passage above a recipe. This proves
only those synthetic pairs, not cross-person task retrieval or safe thresholds.
The model report's multilingual benchmarks likewise are not Evo calibration.

The current tree passed `cargo check --workspace --all-targets --offline --locked
-j1` and the offline workspace test run with an explicitly staged local ONNX
runtime: **1,313 passed, 0 failed, 5 ignored across 61 suite results**. Three
known personal-history tests were filtered. A fresh external-runtime development
bundle at `/private/var/tmp/evo-generalization-dist-qspHKb/Evo.app` passed deep
strict signature verification without being launched. Subsequently, an opt-in
arm64 macOS 14 bundled-runtime build at
`/private/var/tmp/evo-bundled-dist-ipijKy/Evo.app` passed strict signature and
static closure checks. The bundled ONNX Runtime 1.29.0 dylib loaded in an
isolated process and negotiated C API 24; a separate Rust test executable used
only its sibling bundled dylib for the real-weight E5 smoke. The Evo app itself
was not launched or declared release-ready; license/third-party notices were
copied from the immutable upstream source without legal redistribution review.

### Inference invariance

The metamorphic evaluator renders the same synthetic work trace as opaque IDs,
application-like labels, profession labels, and multilingual text. It compares
membership, overlap, standing, and restore roles after normalizing identities,
not literal strings. This prevents a score improvement caused by adding hidden
app or profession rules.

### Evidence and safety

The daemon now detects replacement of `observation.log` by filesystem identity,
including same-length replacements, and clears every derived view before replay.
Execution refuses a locator whose kind, exact subject, or embedded ArtifactId
does not match the selected canonical resource. These are structural safeguards
that do not depend on language, application, or profession.

## What “arbitrary real work” means here

Evo can generalize the *representation and safety rules* to unknown resource
types. It cannot promise semantic understanding when the machine did not
witness enough evidence, when a browser/application exposes no adapter, or when
two works are observationally indistinguishable. In those cases the correct
behavior is clarification, a remembered-but-not-presented hypothesis, or an
explicit unsupported/unverified restore—not a fluent guess.

The next valid evaluation is a locally labeled, opt-in matrix across people,
professions, languages, and applications. Report separately: wrong-work
restores, wrong-resource opens, missed required resources, clarification rate,
unsupported-state rate, time-to-resume, CPU/memory/battery, and privacy
invalidation. Never pool private history into a cloud evaluator.

## Sources

- [W3C PROV-DM](https://www.w3.org/TR/prov-dm/) — domain-agnostic provenance
  model separating entities, activities, agents, time, and derivation.
- [Multilingual E5 Technical Report](https://arxiv.org/html/2402.05672) —
  multilingual contrastive retrieval, role instructions, and MIRACL results.
- [FOCAL: Filtered On-device Continuous Activity Logging](https://arxiv.org/abs/2604.19541) —
  on-device filtering and task-isolated context for interleaved desktop streams.
- [MTEB](https://aclanthology.org/2023.eacl-main.148/) — why embedding quality
  must be measured across task types rather than inferred from one similarity
  probe.
