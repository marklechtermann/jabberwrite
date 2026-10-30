# AGENTS.md

This file provides guidance to AI coding agents when working with code in this repository.

## Project

Jabberwrite is a voice-to-text (dictation) system built primarily for agentic coding and modern AI workflows: speak a prompt and get clean text into a coding agent or editor.

## Guiding principle

Keep it simple. No bloatware, no feature creep, no unnecessary dependencies or configuration surface. When choosing between a smaller solution and a more "complete" one, prefer the smaller one unless the user asks otherwise.

## Status

No code yet. No tech stack, build, lint or test commands exist. Once they do, add them here along with a short architecture overview.

## Workflow: ideas → specs → ADRs → code

All planning lives in `docs/`, written in English. Specs and ADRs are one file per item; ideas are a single collection file. Templates for specs and ADRs are in `docs/templates/`.

| Path           | Holds                                    | Naming                 |
| -------------- | ---------------------------------------- | ---------------------- |
| `docs/ideas.md`| Single inbox of raw ideas, no ceremony   | one `##` section per idea |
| `docs/specs/`  | What to build, ready for implementation  | `NNNN-slug.md`         |
| `docs/adr/`    | Why we chose X over Y                    | `NNNN-slug.md`         |

1. **Idea.** Anyone (human or agent) may add an idea to `docs/ideas.md` at any time as a new `##` section with a few sentences. Never create one file per idea. When the user mentions an idea in passing, capture it there.
2. **Spec.** Only the user decides that an idea becomes a spec. Agents may draft the spec when asked, but never promote an idea on their own. On promotion, create `docs/specs/NNNN-slug.md` and remove the idea's section from `docs/ideas.md`: once an idea is formulated as a spec, it no longer lives in the ideas file. Also remove ideas that are already implemented.
3. **ADR.** When a spec requires a real decision (engine, language, library, architecture), record it as an ADR. ADRs are never edited after acceptance except to set `status: superseded by NNNN`; a changed decision gets a new ADR.
4. **Implementation.** One spec per branch, named `spec/NNNN-slug`, merged through a PR that references the spec. Keep the spec's task checklist and `status` up to date while working.

### Rules for agents

- Spec statuses: `draft` → `accepted` → `in-progress` → `done`. Only pick up specs with `status: accepted`; set `in-progress` when you start.
- Before implementing, read the spec and every ADR it links. Do not reopen decisions recorded in an accepted ADR; propose a new ADR instead.
- Numbers are sequential per directory. Take the next free number on `main` at the time you create the file.
- When working in parallel with other agents, use a separate git worktree per spec branch.
- Stay within the spec's scope. Anything new you notice goes into `docs/ideas.md`, not into the current branch.

### License compliance

- Whenever you add, remove or update a dependency (`src-tauri/Cargo.toml`, `package.json`, lockfiles), run `npm run licenses` and commit the regenerated `ACKNOWLEDGEMENTS.md` (the single file with all third-party licenses). It feeds the Licenses topic in the settings window, and CI fails when it is stale.
- Components outside Cargo and npm (downloaded binaries, models, vendored code) go into the `manual` list in `scripts/generate-licenses.mjs`, with their license text in `third-party/`. This includes native libraries pulled in by a crate's build script (check what it downloads or links).
- `npm run licenses` fails on licenses outside its allow-list (permissive, Apache-2.0-compatible). Do not extend the allow-list yourself; ask the user.
- Check that the license of a new dependency is compatible with Apache-2.0 (ADR 0006). Ask the user before adding anything copyleft.
