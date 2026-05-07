# AGENTS.md

Behavioral and project guidelines for AI coding agents working on this repository.

These instructions are intentionally strict. This project is expected to be developed by repeated AI-agent sessions, so preserving context, design intent, and work history is part of the implementation work.

## 1. Think Before Coding

Do not assume silently. Do not hide confusion.

Before implementing:

- State your assumptions when the task is ambiguous.
- If multiple interpretations exist, surface them before choosing.
- Prefer the simplest design that satisfies the requirement.
- Push back when a requested change would add avoidable complexity.
- Do not rewrite unrelated code.
- Do not preserve legacy structure merely because it already exists.

For trivial edits, use judgment and proceed directly.

## 2. Read Project Notes First

Before making non-trivial changes, inspect the `docs/` directory.

At minimum, check these files if they exist:

- `docs/todo.md` — current task list, known gaps, next steps.
- `docs/design.md` — architecture, design constraints, module boundaries.
- `docs/decisions.md` — important decisions and their rationale.
- `docs/work-log.md` — append-only log of previous agent work.

If these files do not exist and your task would benefit from them, create them.

Do not rely only on source code structure. This project may contain recent decisions that are documented in `docs/` but not yet fully reflected in code.

## 3. Keep a Work Log

Every meaningful change must leave a short entry in `docs/work-log.md`.

The log is not a diary. It is a continuity mechanism for future agents and humans.

Append to the log after completing a coherent unit of work, such as:

- scaffolding a new app/module/crate
- changing architecture
- adding a feature
- fixing a bug
- changing command/API behavior
- adding or changing tests
- discovering an important constraint
- intentionally deferring work

Rules:

- Append only. Do not rewrite old log entries unless correcting factual errors.
- Keep entries concise but specific.
- Mention files/modules touched when useful.
- Record failed attempts if they reveal useful constraints.
- Record commands used for validation, especially tests/builds/lints.
- If validation was not run, explicitly say why.

## 4. Maintain Design Notes

When a change affects architecture or long-term maintainability, update `docs/design.md` or `docs/decisions.md`.

Examples of changes that require documentation:

- crate/module boundaries
- Tauri command structure
- frontend/backend responsibility split
- job/progress/cancellation model
- file scanning or extraction pipeline
- database/storage layout
- error handling strategy
- platform-specific behavior
- security-sensitive file system behavior

Do not bury important design rationale only in code comments.

## 5. Respect the Intended Architecture

This project should keep business logic out of the GUI adapter layer.

Preferred structure:

- core logic belongs in a reusable Rust crate
- CLI behavior belongs in a thin CLI crate
- Tauri code should be an adapter between frontend and Rust core
- frontend code should handle UI, state presentation, and user interaction
- long-running jobs should expose progress, cancellation, and structured errors

Avoid placing archive extraction, scanning, metadata parsing, or library management logic directly inside Tauri command handlers.

Tauri command handlers should mostly:

- validate input
- call core services
- translate results/errors
- emit progress/events
- manage job lifecycle

## 6. Do Not Over-Engineer

Favor boring, explicit, maintainable code.

Avoid:

- speculative abstraction
- generic frameworks before there are multiple real use cases
- complex dependency injection
- hidden global state
- clever macros
- large refactors unrelated to the task
- preserving old compatibility layers unless explicitly required

If a simple direct implementation is enough, use it.

## 7. Keep Changes Reviewable

Make changes in coherent steps.

When possible:

- separate mechanical moves from behavior changes
- separate scaffolding from feature implementation
- separate formatting from logic changes
- avoid mixing large refactors with bug fixes

Do not touch unrelated files just to “clean up” while solving another task.

## 8. Validate Changes

After modifying code, run the most relevant validation available.

Prefer, as applicable:

- `cargo test`
- `cargo ch[118;1:3ueck`
- `cargo clippy`
- frontend typecheck
- frontend lint
- app build
- targeted unit tests
- targeted CLI/manual smoke tests

If a command fails, investigate before declaring the task complete.

If validation cannot be run because the project is still being scaffolded or dependencies are missing, record that in `docs/work-log.md`.

## 9. Preserve User Intent

When implementing a user request, distinguish between:

- what the user explicitly asked for
- what is necessary to make it work
- what would merely be nice to have

Do the first two. Avoid the third unless it is very small and clearly beneficial.

If the requested task conflicts with existing design notes, stop and explain the conflict before making broad changes.

## 10. Leave the Repository Better Oriented

At the end of a non-trivial task:

- update `docs/todo.md` if the task changes the next steps
- update `docs/design.md` or `docs/decisions.md` if architecture changed
- append to `docs/work-log.md`
- mention validation performed
- mention known remaining issues

Future agents should be able to resume work by reading `AGENTS.md` and `docs/`.

