# Codex repository instructions

Before making changes, read:

1. `IMPLEMENTATION.md`
2. `CODEX.md`
3. `ARCHITECTURE.md`
4. `DEVELOPERS.md`
5. `USAGE.md`
6. `README.md`

`IMPLEMENTATION.md` is the authoritative architectural roadmap and acceptance
criteria.

`CODEX.md` is the live execution journal. Maintain it throughout every task.

## Progress tracking

Before editing code:

- Read the current state in `CODEX.md`.
- Verify that its claims still match the repository.
- Record the current task, baseline commit, and validation status.
- Identify the next incomplete milestone from `IMPLEMENTATION.md`.

After each coherent milestone:

- Update `CODEX.md`.
- Record files and public APIs changed.
- Record decisions and their rationale.
- Record tests and validation commands run.
- Record failures, warnings, and unresolved risks.
- Record the exact next task.
- Leave the repository compiling and tested.

Before ending any session:

- Update `CODEX.md`, even when the task failed or is incomplete.
- Never claim a milestone is complete without implementation, tests, and
  documentation.
- Never erase unresolved problems from the log.
- Move durable architectural decisions into `docs/adr/` and link them from
  `CODEX.md`.
- Keep detailed execution history out of `IMPLEMENTATION.md`.

`IMPLEMENTATION.md` may have milestone checkboxes updated, but its architectural
content should not be rewritten merely to match an incomplete implementation.