# Local synthetic thread evaluation

Run the held-out evaluator with:

```sh
cargo run -p evo-threads --bin evaluation
```

`crates/evo-threads/src/bin/evaluation.rs` constructs all events in memory and
replays them through the public `Engine` API. It does not read personal history,
load a model, access the network, or inspect the filesystem.

The fixtures cover:

- two overlapping works sharing a planning resource;
- multilingual resource labels and opaque non-Latin locators;
- a reading-only work formed by cross-day recurrence;
- unrelated and passive attention that must not become work or restore targets;
- temporal gaps that close sittings; and
- a stable artifact observed through renamed machine/application surfaces.

The report keeps four metrics independent:

- **grouping usefulness**: expected same-work relations and work roots recovered;
- **grouping safety**: expected separations and unrelated-resource exclusions;
- **restoration usefulness**: required current surfaces present in `restore_set`;
- **restoration safety**: stale, shared, ambient, unrelated, and over-cap surfaces
  absent from `restore_set`.

It also checks replay determinism. A failed synthetic contract exits non-zero and
should be reported to the owning engine workstream; this harness does not change
production heuristics to improve its score.

## Metamorphic engagement evaluation

Run the label-invariance evaluator with:

```sh
cargo run -p evo-engagement --bin metamorphic_evaluation
```

The source is `crates/evo-engagement/src/bin/metamorphic_evaluation.rs`. It
constructs one in-memory trace containing:

- two independently interleaved works with a resource shared by both;
- a reading-only work with no production, save, or commit event;
- an ambient attention surface and unrelated attention that must not be opened
  or claimed as work; and
- three sittings per work so return, depth, standing, and continuation roles are
  observable.

The exact same semantic trace is rendered with opaque identifiers,
app/process-like labels, and profession/multilingual text. Each result is
normalized back to semantic slots before comparison. The evaluator compares
memberships, overlap, independent-work separation, `Standing`, and restore
opening roles; it never asserts a literal resource name. It also reverses each
trace before reconstruction to check input-order determinism.

Expected baseline output on this repository:

```text
opaque identifiers                engagements=5 works=3 A=1 B=1 reading=1
app/process-like labels           engagements=5 works=3 A=1 B=1 reading=1
profession and multilingual text  engagements=5 works=3 A=1 B=1 reading=1
metamorphic invariants: 38/38 (100.0%)
result: PASS (synthetic invariants only; not real-user accuracy)
```

This is a local-only synthetic gate: it does not read personal data, the
filesystem, network, model files, or a wall clock. A failure is evidence for
the owning inference workstream; do not modify production heuristics merely to
raise this score.
