---
name: commit
description: Generate commit messages following project conventions. Use when committing changes or reviewing staged files.
allowed-tools: Bash(git:*)
---

# Commit Message Generator

## Instructions

1. Run `git status` to check for all changes (staged, unstaged, untracked)
2. If there are unstaged or untracked files, run `git add -A` to stage all changes
3. Run `git diff --staged` to see what will be committed
4. If no changes to commit, inform the user and stop
5. Generate a commit message following the rules below
6. Run `git commit -m "message"` immediately (no confirmation needed)

### Format
- Line 1: Summary (max 50 chars, imperative mood)
- Line 2: Blank
- Line 3+: Description (optional, explain what and why)

### Rules
- Write in English
- Use imperative mood: "Add", "Fix", "Change" (not "Added", "Fixed")
- NO prefixes like feat:, fix:, docs:
- Be specific, avoid vague messages

### Examples

❌ Bad
- Fix
- Bug fix
- Various changes

✅ Good
- Change audio capture buffer size to 20ms
- Add retry logic for P2P connection failures
- Remove unused audio codec parameters
