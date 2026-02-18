# AGENTS.md
Guidance for coding agents (Codex, Claude Code, etc.) working in this repository.

## Read First
- **Read `CLAUDE.md` before doing any work.** Treat it as the canonical repo guide (architecture, constraints, workflows, and .agent/ conventions).
- If `AGENTS.md` and `CLAUDE.md` ever disagree, **follow `CLAUDE.md`**.

## GitHub Auth (Before Git Operations)
This repo pushes to GitHub as `markbookdev`. Before any git operations that touch the remote (push, fetch with auth, creating PRs, etc.):

1. Switch GitHub CLI to the correct account:
   - `gh auth switch -u markbookdev`
2. Confirm it’s active:
   - `gh auth status`
   - Ensure it shows: `Active account: true` for `markbookdev`.
3. Then proceed with git operations (`git push`, etc.).

If a push fails with 403/permission errors, re-run the steps above and verify the remote URL with:
- `git remote -v`

