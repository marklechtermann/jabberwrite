# AGENTS.md

This file provides guidance to AI coding agents when working with code in this repository.

## Project

Jabberwrite is a voice-to-text (dictation) system built primarily for agentic coding and modern AI workflows: speak a prompt and get clean text into a coding agent or editor.

## Finish flow (handover)

All implementation work happens in a separate git worktree on its own branch, never directly on `main`.

When an implementation is complete, ask the user: **"Run the full finish flow?"** (commit, PR, auto-merge, cleanup). If the user says yes, or says "mach alles fertig", "mach alles sauber", "clean up", "finish" or "handover", run every step below without asking again. Otherwise ask before each step.

1. **Commit** all changes on the worktree branch.
2. **Push**: `git push -u origin <branch>`.
3. **Open the PR** with `gh pr create --fill` (reference the spec or issue if there is one). If `gh` is not available, stop here and give the user the compare URL.
4. **Auto-merge**: `gh pr merge <n> --auto --squash`. GitHub merges once checks are green and deletes the remote branch itself.
5. **Wait for the merge**: `gh pr checks <n> --watch`, then confirm with `gh pr view <n> --json state -q .state` that it is `MERGED`. If checks fail or the PR is blocked, stop and report; do not clean up.
6. **Clean up locally**, run from the main worktree, not from the one being removed:
   ```bash
   git worktree remove <worktree-path>
   git branch -D <branch>   # -D: squash merges are not seen as merged by -d
   git checkout main && git pull --prune
   ```
   Only delete the branch after step 5 confirmed `MERGED`.

If the PR was already opened earlier, "clean up" resumes at the first step not yet done.

## Agent skills

### Issue tracker

Issues live as local markdown files under `.scratch/<feature-slug>/`. See `docs/agents/issue-tracker.md`.

### Domain docs

Single-context: one `GLOSSARY.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.
