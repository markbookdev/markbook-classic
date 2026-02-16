# .agent/ Folder Structure
This folder supports Claude Code workflows and session continuity.

| Folder | Purpose |
|--------|---------|
| `conversations/` | Archived conversation summaries for context handoff |
| `prompts/` | Continuation prompts for starting new sessions |
| `tasks/` | Tracking for long-running or complex tasks |
| `product/` | Product docs, specs, and requirements |
| `important/` | Critical insights, aha moments, hard-won lessons |
| `decisions/` | Architectural Decision Records (ADRs) |
| `scratch/` | Temporary working notes (can be gitignored) |

## Security Reminder
Never commit secrets, API keys, passwords, or credentials to any file in this folder.
When archiving conversations, always scan for and redact sensitive information.

