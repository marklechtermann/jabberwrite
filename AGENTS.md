# AGENTS.md

This file provides guidance to AI coding agents when working with code in this repository.

## Project

Jabberwrite is a voice-to-text (dictation) system built primarily for agentic coding and modern AI workflows: speak a prompt and get clean text into a coding agent or editor.

## Guiding principle

Keep it simple. No bloatware, no feature creep, no unnecessary dependencies or configuration surface. When choosing between a smaller solution and a more "complete" one, prefer the smaller one unless the user asks otherwise.

## Status

No code yet. No tech stack, build, lint or test commands exist. Once they do, add them here along with a short architecture overview.

## Workflow: ideas → specs → ADRs → code

All planning lives in `docs/`, one file per item, written in English. Templates are in `docs/templates/`.

| Directory      | Holds                                    | Naming                 |
| -------------- | ---------------------------------------- | ---------------------- |
| `docs/ideas/`  | Inbox of raw ideas, no ceremony          | `slug.md` (no number)  |
| `docs/specs/`  | What to build, ready for implementation  | `NNNN-slug.md`         |
| `docs/adr/`    | Why we chose X over Y                    | `NNNN-slug.md`         |

1. **Idea.** Anyone (human or agent) may add an idea to `docs/ideas/` at any time. When the user mentions an idea in passing, capture it there.
2. **Spec.** Only the user decides that an idea becomes a spec. Agents may draft the spec when asked, but never promote an idea on their own. On promotion, create `docs/specs/NNNN-slug.md` and delete the idea file.
3. **ADR.** When a spec requires a real decision (engine, language, library, architecture), record it as an ADR. ADRs are never edited after acceptance except to set `status: superseded by NNNN`; a changed decision gets a new ADR.
4. **Implementation.** One spec per branch, named `spec/NNNN-slug`, merged through a PR that references the spec. Keep the spec's task checklist and `status` up to date while working.

### Rules for agents

- Spec statuses: `draft` → `accepted` → `in-progress` → `done`. Only pick up specs with `status: accepted`; set `in-progress` when you start.
- Before implementing, read the spec and every ADR it links. Do not reopen decisions recorded in an accepted ADR; propose a new ADR instead.
- Numbers are sequential per directory. Take the next free number on `main` at the time you create the file.
- When working in parallel with other agents, use a separate git worktree per spec branch.
- Stay within the spec's scope. Anything new you notice goes into `docs/ideas/`, not into the current branch.
