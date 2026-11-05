# The jurisdiction layer — what exists, what doesn't

This note exists because the obvious pitch for Evo is "it learns what a work
means to you." That pitch is wrong, and it would cost us the room. Here is
the truth, the reason it is stronger, and the exact words to use.

## It is not learning. It is legislation.

The engine has no weights, no fine-tuning and no feedback loop. What it has is
a `Declaration` — the person's own word, recorded as a first-class event and
applied deterministically from then on. Six kinds, all real, all in
`crates/evo-threads/src/types.rs`:

| Declaration | What the person is saying | What the engine does |
|---|---|---|
| `SameWork { a, b }` | "These belong together." | The **only** merge path in the system. Fuses the two threads. |
| `SeparateWork { a, b }` | "These are separate." | A **standing constraint**: no future inference may ever place them together. |
| `NotMine { resource }` | "This is not my work." | Permanent quarantine. Stripped from every view, never evidence again. |
| `Classified { resource, class }` | "This surface is of a class" — media, not work. | Gates founding and titling only. Keeps whatever attention it earned. |
| `WorkDismissed { work }` | "Don't show this on my default surface." | Evidence retained, work still retrievable. |
| `MemberState { work, member, state }` | A lifecycle correction for one member. | Applies to that member only. |

`SeparateWork` is the important one. It is not a preference that tilts a
score — it is a prohibition. From `assign.rs:910`:

> *"Recorded: … are separate works; inference will never put them together."*

That is a jurisdiction. The person legislates; the engine adjudicates; the
constraint outlives the session and cannot be overridden by evidence.

## Why this beats "it learns"

1. **A learning claim is unfalsifiable.** A judge cannot check it in three
   minutes, so they discount it. A declaration is checkable: the person said
   it, the log recorded it, the answer changed.
2. **Learning contradicts our only moat.** Our claim is that the same input
   produces the same output twice. A model that adapts is precisely what we
   are not. Say we learn and the whole architecture story collapses into
   "another AI product."
3. **Learning is what everyone else is selling.** 90% of the room will say
   their product gets better over time. Nobody will say "this one refuses,
   and remembers your refusal forever."

## The honest model: three states, no fourth

Every resource in Evo is in exactly one of three states:

```
  DERIVED    the engine placed it, from evidence it witnessed
  DECLARED   the person overrode or confirmed it, and the log holds that
  REFUSED    the engine declined to place it at all
```

There is no fourth state. Nothing is inferred that was not witnessed, and
nothing is asserted that was not earned. That is a legal system, not a model.

## What is on screen today — say this precisely

| State | Visible in the demo? |
|---|---|
| **Derived** | Yes. `2 of 8 claimed` in the header of `evo_ui.html`. |
| **Refused** | Yes. The six greyed rows, marked `refused: engine would not assert this` in `list_works`. |
| **Declared** | **Not yet.** The engine implements all six declarations; the service exposes no endpoint for them. |

The two buttons in `boundary.html` demonstrate the *gate*, not a declaration
write. Pressing "Declare them the same work" simulates the engine fusing two
works and shows the verifier failing the build. It does not write to the
engine. Do not claim it does.

## The roadmap answer, if asked "does it learn?"

> "No, and that's deliberate. It doesn't learn — the person legislates. There
> are six kinds of declaration in the engine: this is the same work, these are
> separate, this isn't mine, this surface is of a class, don't show this work.
> Once you declare two works separate, nothing in inference can ever put them
> back together — it's a standing prohibition, not a preference. Learning would
> make the boundary unreproducible, and an unreproducible boundary is not
> something you'd give an agent permission to act on."

## If there is time to build it (not before the recording)

The smallest honest addition, service layer only, no Rust:

```
POST /declare   {"a": "<resource>", "b": "<resource>", "kind": "separate"}
  -> appends to declarations.json
  -> every subsequent /ask and /find_work honours it
  -> the answer reports: "declared: a and b are separate"
```

Then the demo beat is: ask a question, get two candidate works, say "these are
separate", ask again, and the rejected work is now permanently excluded.

Two rules for that build:

1. It must persist to a file and survive a restart, or it is theatre.
2. It must be described as *the service recording your declaration*, with the
   engine's declaration path named as the production mechanism. Do not imply
   the running engine has been replayed.

**Do not start this before the backup recording exists.**
