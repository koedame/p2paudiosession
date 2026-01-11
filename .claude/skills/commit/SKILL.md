---
name: commit
description: Generate commit messages following project conventions. Use when committing changes or reviewing staged files.
allowed-tools: Bash(git:*), Bash(cargo *), Read, Edit, Glob, Grep
---

# Commit Message Generator

## Instructions

1. Run `git status` to check for all changes (staged, unstaged, untracked)
2. If there are unstaged or untracked files, run `git add -A` to stage all changes
3. Run `git diff --staged` to see what will be committed
4. If no changes to commit, inform the user and stop

### Pre-commit Checks

5. Run lint and test checks before committing:
   - Run `cargo fmt --check` to check formatting
   - Run `cargo clippy -- -D warnings` to check for lint errors
   - Run `cargo test` to run tests

6. If any checks fail:
   - For formatting issues: Run `cargo fmt` to auto-fix
   - For clippy warnings: Read the affected files and fix the issues
   - For test failures: Investigate and fix the failing tests
   - After fixes, run `git add -A` to stage the fixes
   - Re-run the failed checks to verify fixes

7. Once all checks pass, generate a commit message following the rules below
8. Run `git commit -m "message"` immediately (no confirmation needed)

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
