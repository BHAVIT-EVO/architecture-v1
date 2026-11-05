<div align="center">

# evo

### Pick up where you left off.

**Your work doesn't live in one app. Why should your context?**

[The idea](#the-idea) · [How Evo works](#how-evo-works) · [For AI agents](#a-better-starting-point-for-ai-agents) · [Explore the code](#explore-the-code)

</div>

---

You were working on something important. A document, a few browser pages, maybe a folder and an app or two. Then a message came in. A meeting happened. The day moved on.

The next time you sit down, the work is still there. But first you have to find the pieces, remember how they fit together, and work out where to begin.

**Evo is being built to make that return easier.**

It's an early, local-first project that explores a simple idea: instead of making you organize your life around individual apps and files, your computer should help you find your way back to the *work* connecting them.

> **Continue the work. Skip the archaeology.**

## The idea

Think about planning a trip, researching a topic, or building a product. Each is one activity, but its pieces can be scattered across your computer. Evo looks for relationships between the activity signals and resources it can access, and uses them to form candidate **bodies of work**.

The goal is not to preserve a perfect recording of your day. It's to make the relevant pieces easier to find when you need them again.

```text
  A document       A few pages       An app or folder
       \                |                 /
        \               |                /
         ───────  a piece of work  ───────
                        |
                 return to it
```

Whether you're switching between projects, studying for an exam, or picking up a personal idea after a few days away, the ambition is the same: **less time reconstructing context, more time actually doing the thing.**

## How Evo works

Evo's macOS-focused engineering build explores a pipeline like this:

1. **Observe supported activity signals.** Depending on what's enabled and available, these can include things like app focus, file activity and URLs.
2. **Find relationships.** An engine processes observations and builds candidate groupings of related work and resources.
3. **Help you return.** The desktop experience aims to surface a piece of work and reopen supported resources so you can continue.

Evo works with evidence it can access—not a complete record of everything you thought or did. Groupings can be incomplete or wrong; a link to a document is not the same as knowing the decision you made while reading it. Reopening a resource is not necessarily restoring every tab, window or application state exactly as it was.

## A better starting point for AI agents

Now imagine asking an AI agent: *“Help me continue that project.”*

The agent may be capable of writing, searching or taking actions, but it still needs to know **which project you mean** and **which context is relevant**. Otherwise, you're back to explaining the work from scratch.

Our longer-term direction is for a person to choose whether to share relevant Evo context with an agent they use. In a possible future workflow, an agent could request context for a selected activity, receive a limited set of relevant resources and evidence, and ask for clarification when the answer isn't clear.

This is a direction, **not a claim of a finished agent platform**. An early query prototype has been explored, but production integrations, granular permissions and safe action controls still need to be built and validated. A protocol alone doesn't enforce a security boundary; sending context to a cloud model would also mean that selected data leaves the device.

## Privacy is part of the product

Evo's design rule is straightforward: **no screenshots or screen recordings, even temporarily.** No image capture is needed as a workaround.

That doesn't make the remaining data automatically harmless. File names, window titles and URLs can still be sensitive. Local-first design, careful permissions, data minimization and transparent controls matter—and they need to be implemented and tested, not just promised.

## Where we are

Evo is in **active, early development**. There is a macOS MVP aimed at helping a person return to a piece of work, alongside an evolving Rust codebase for observation, work modeling and restoration. This repository is an engineering build, not a polished general release; behavior and setup can change.

What we're working toward:

- More reliable work grouping and a useful, understandable return experience.
- Clear handling of uncertainty and ways for people to correct mistakes.
- Privacy and security controls appropriate for real-world use.
- Carefully scoped, user-directed context for AI agents.
- Learning from repeated use across more people and activities.

The broader vision is for people everywhere, across work and personal life. The practical starting point is proving the experience on macOS. Windows, mobile and cross-device support are not promised as shipped capabilities.

## Explore the code

This is a Rust workspace. A few places to start:

| Area | What to explore |
| --- | --- |
| [`evo-capture`](crates/evo-capture/) | Structured observation and platform adapters |
| [`evo-threads`](crates/evo-threads/) | Work grouping and retrieval logic |
| [`evo-restoration`](crates/evo-restoration/) | Restoration-related components |
| [`evo-desktop`](crates/evo-desktop/) | The desktop application |
| [Architecture](docs/architecture/ARCHITECTURE.md) | The system's architecture |
| [Work model](docs/architecture/WORK-MODEL.md) | How the project thinks about work |
| [Constitution](docs/constitution/CONSTITUTION.md) | Principles and constraints |

If you're exploring the source, start with the architecture and the individual crates. Building or running the macOS app requires a Mac and a Rust toolchain; system permissions and setup depend on the build. This project is changing quickly, so check the scripts and code before running them. The application has not been verified on every machine or configuration.

---

<div align="center">

**Your work is more than a collection of tabs.**

Evo is an exploration of what happens when your computer starts treating it that way.

</div>
