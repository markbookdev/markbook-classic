# CLAUDE.md
This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

*Last Updated:* 2026-02-14

## Project Overview
MarkBook Classic is an offline-first desktop rewrite of the legacy MarkBook Windows (VB6) application used by teachers for class management and gradebook workflows (students, assessments, mark sets, attendance, reporting, exports).

This repository is the desktop implementation using Electron + React + TypeScript managed with Bun workspaces, plus a Rust sidecar (`markbookd`) that owns local persistence (SQLite), filesystem import/export (legacy MarkBook file formats and MB Exchange CSV), and report model generation. Reports are rendered to PDF using Chromium via `webContents.printToPDF` for consistent output across Windows and macOS.

## Commands
```bash
# [FILL IN: Common development commands]

bun install                 # Install dependencies
bun run dev                 # Start desktop dev (Vite + Electron)
bun run build               # Build desktop artifacts (dev packaging)
bun run sidecar:build:debug # Build Rust sidecar (debug)
bun run sidecar:build:release # Build Rust sidecar (release)
```

## Git Commits
Never add Claude attribution or co-author lines to commits.

## Architecture
High level:
- Renderer (React) never touches the filesystem or database directly.
- Electron main process:
  - launches the Rust sidecar and forwards typed IPC requests
  - owns OS dialogs (select workspace, pick legacy folders)
  - owns PDF export pipeline (hidden BrowserWindow + `printToPDF`)
- Rust sidecar (`rust/markbookd`):
  - stores all user data in SQLite in a user-selected workspace folder
  - imports legacy MarkBook class folders (e.g. `CL*.Y25`, `*.Y25`, `.RMK`, etc.)
  - exports/imports MB Exchange CSV (future)

Repo layout:
- `apps/desktop`: Electron app (main + preload + React renderer)
- `packages/schema`: shared Zod schemas and IPC types (source of truth for payloads)
- `packages/core`: shared pure TypeScript logic (calculations/validators; reused with web later)
- `packages/reports`: HTML/CSS report templates (used for PDF export)
- `rust/markbookd`: Rust sidecar (SQLite + import/export)

## Key Patterns
- Security boundary: renderer talks only to a narrow preload API; all privileged actions go through Electron main -> Rust sidecar.
- IPC: newline-delimited JSON request/response with `id`, `method`, `params`; errors are structured.
- Data ownership: SQLite is the source of truth; UI holds view state only.
- Reports: generate a report model (data) first, then render HTML/CSS, then PDF.

## Agent Workflow (.agent/ folder)

### Security - CRITICAL
NEVER store sensitive information in any `.agent/` files:
- API keys, tokens, or secrets
- Passwords or credentials
- Connection strings with embedded credentials
- Private keys or certificates
- Environment variable values that contain secrets
- Client names or identifying information (if confidential)

When archiving conversations or creating prompts, actively scan for and redact any sensitive information that may have been discussed. Replace with placeholders like `[REDACTED]`, `[API_KEY_HERE]`, or `[CLIENT_NAME]`.

### Conversations (`.agent/conversations/`)
When asked to "archive conversation" or "archive convo":
1. Create a markdown file with naming: `YYYY-MM-DD-<short-descriptor>.md`
2. Before writing, review for secrets/credentials and redact them
3. Write an extremely detailed summary of the conversation including:
   - What was discussed and decided
   - All code changes made (files modified/created, what changed)
   - Current state of the work
   - Any open questions or unresolved issues
   - Key context that would be lost without this summary
4. Always include a "Related Files" section listing exact file paths that were created, modified, or are relevant to the work

Example structure:
```md
# YYYY-MM-DD - [Descriptor]
## Summary
[High-level overview]
## What Was Done
- [Detailed list of changes]
## Related Files
- `src/components/Auth.tsx` - Modified: added logout handler
- `src/api/auth.ts` - Created: new auth API client
- `tests/auth.test.ts` - Modified: added tests for logout
## Decisions Made
- [Key decisions and rationale]
## Open Questions / Next Steps
- [What remains to be done]
## Context for Next Session
- [Important context that shouldn't be lost]
```

### Prompts (`.agent/prompts/`)
When asked to create a "new prompt" or "continuation prompt" (usually after archiving):
1. Create a markdown file: `YYYY-MM-DD-<short-descriptor>-prompt.md`
2. Before writing, review for secrets/credentials and redact them
3. Include:
   - Brief reference to the archived conversation (file path, not full content)
   - Detailed instructions for the next session
   - Current state summary (minimal, to save context window)
   - Specific next steps or goals
   - Any additional instructions provided by the user

### Tasks (`.agent/tasks/`)
For long-running or complex tasks:
1. Create a task file: `<task-name>.md`
2. Track:
   - Task overview and goals
   - Breakdown of subtasks/phases
   - Progress status for each part
   - Blockers or dependencies
   - Notes and decisions made along the way
3. Reference this file in prompts when continuing work across sessions
4. Update the task file as work progresses

### Important (`.agent/important/`)
For critical insights and knowledge worth preserving long-term:
1. Create a markdown file: `YYYY-MM-DD-<short-descriptor>.md`
2. Use this folder when:
   - You discover an "aha moment" or breakthrough understanding
   - You solve a tricky bug and want to document the root cause
   - You find an important pattern, gotcha, or non-obvious behavior
   - You learn something about the codebase that future sessions should know
   - You encounter a hard-won lesson that shouldn't be forgotten
3. Format each entry with:
   - Clear title describing the insight
   - Context: what problem or situation led to this discovery
   - The insight itself: what was learned
   - Why it matters: how this knowledge helps future work
   - Any related files or code references

### Decisions (`.agent/decisions/`)
For architectural decision records (ADRs):
1. Create a markdown file: `YYYY-MM-DD-<decision-topic>.md`
2. Use when making significant technical choices that should be documented
3. Format each entry with:
   - Title: Short description of the decision
   - Status: Proposed / Accepted / Deprecated / Superseded
   - Context: What situation or problem prompted this decision?
   - Options Considered: List alternatives that were evaluated
   - Decision: What was chosen and why
   - Consequences: What are the tradeoffs? What does this enable or prevent?

### Scratch (`.agent/scratch/`)
For temporary working notes:
- Use for rough notes, debugging logs, or temporary context
- These files don't need to be preserved long-term
- Can be excluded from git if desired
- Clean up periodically

### Product (`.agent/product/`)
Product documentation and specifications:
- Contains product descriptions, use cases, user stories, and feature specs
- Reference these files to understand product context and goals
- Update when product scope or requirements change
- Use as source of truth for what the product should do and why
