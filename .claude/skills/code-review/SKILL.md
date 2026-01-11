---
name: code-review
description: Review code changes for quality, spec compliance, and OSS readiness. Use when reviewing staged changes, PRs, or specific files for issues.
allowed-tools: Read, Glob, Grep, Bash(git:*), Bash(cargo:*)
---

# Code Review

## Instructions

### Target Identification

1. Determine review target:
   - If argument is a file path: Review that specific file
   - If argument is a PR number: Run `git diff main...HEAD` (or appropriate base branch)
   - If no argument: Run `git diff --staged` to review staged changes
   - If no staged changes: Run `git diff` to review unstaged changes

2. If no changes found, inform user and stop

### Review Process

3. For each changed file, apply the 5 review perspectives in order:
   1. Root Cause Analysis (本質的対応)
   2. Spec Compliance (仕様準拠)
   3. Test Quality (テスト品質)
   4. OSS Readiness (OSS公開適合性)
   5. Guideline Compliance (ガイドライン準拠)

4. Output results in the format specified below

---

## Review Perspectives

### 1. Root Cause Analysis (本質的対応)

Detect symptomatic fixes that don't address root causes.

#### Red Flags

| Pattern | Issue | Question to Ask |
|---------|-------|-----------------|
| Adding `.unwrap_or_default()` to hide errors | Error suppression | Why is this error occurring? Should it be handled? |
| Adding `if` guards without understanding why | Defensive coding | What condition causes this to be needed? |
| Copy-pasting similar code blocks | Code duplication | Should this be abstracted? |
| Adding `sleep()` or delays | Timing hack | What race condition or async issue exists? |
| Adding `#[allow(...)]` to suppress warnings | Warning suppression | Why does this warning occur? Is it valid? |
| Null/Option checks proliferating | Unclear ownership | Who owns this data? When can it be None? |
| try/catch wrapping entire functions | Broad error handling | Which specific operations can fail? |

#### Questions to Raise

- "This change treats the symptom. What is the underlying cause?"
- "Why does this condition need to be checked here?"
- "Is this the right layer to handle this issue?"

---

### 2. Spec Compliance (仕様準拠)

Verify implementation matches specifications, not vice versa.

#### Check Process

1. Identify which spec the code relates to:
   - `docs-spec/api/*.md` - API specifications
   - `docs-spec/architecture.md` - Architecture decisions
   - `docs-spec/adr/*.md` - Architecture Decision Records
   - `docs-spec/behavior/*.feature` - BDD scenarios

2. Read the relevant spec files

3. Compare implementation against spec:

| Check | Red Flag |
|-------|----------|
| Function signature | Parameters added/removed without spec update |
| Error handling | Error types differ from spec |
| Threading model | Thread constraints violated |
| Data structures | Fields added/removed without spec update |
| Behavior | Logic differs from BDD scenarios |

#### Critical Questions

- "Does this change require a spec update?"
- "Is the implementation bending the spec for convenience?"
- "Should the spec be changed, or the implementation?"

If spec change is warranted, recommend creating/updating ADR to document why.

---

### 3. Test Quality (テスト品質)

Ensure tests verify intended behavior, not accidental implementation details.

#### Accidental vs Intentional Specification

Tests should verify **what the code should do** (spec), not **what it happens to do** (implementation).

| Accidental (Bad) | Intentional (Good) |
|------------------|-------------------|
| Testing exact error message strings | Testing error type/category |
| Testing internal state after operation | Testing observable behavior/output |
| Testing execution order of internal calls | Testing final result is correct |
| Asserting specific timing values | Asserting timing within acceptable range |
| Testing private helper function behavior | Testing public API contract |

#### Red Flags in Test Code

| Pattern | Issue | Question to Ask |
|---------|-------|-----------------|
| `assert_eq!(result, "Error at line 42")` | Brittle string matching | Is the exact message part of the API contract? |
| Testing internal struct field values | Coupling to implementation | Should this be testing the public interface? |
| Mock returning hardcoded implementation detail | Fake implementation | Does this test actual behavior or a fake? |
| Test passes only with specific timing | Timing dependency | Is this testing race conditions or flaky? |
| Copying production code logic into test | Tautological test | Is this just replicating bugs? |
| `#[ignore]` without explanation | Hidden failures | Why is this test disabled? |

#### Test Intent Verification

For each test, ask:

1. **What spec does this test verify?**
   - Map to `docs-spec/behavior/*.feature` scenario
   - If no spec exists, should one be created?

2. **Would this test fail if the spec is violated?**
   - False positive: Test passes when behavior is wrong
   - False negative: Test fails when behavior is correct

3. **Would this test still pass after valid refactoring?**
   - If implementation changes but behavior stays same, test should pass
   - If test breaks on refactor, it's testing implementation

4. **Is the assertion meaningful?**
   - `assert!(result.is_ok())` - Too weak, what should `result` contain?
   - `assert_eq!(result, expected)` - Good if `expected` is from spec

#### Test Coverage Quality

| Check | Requirement |
|-------|-------------|
| Happy path | Basic success scenario from BDD |
| Error cases | Each error type in spec should have test |
| Edge cases | Boundary values, empty inputs, limits |
| Concurrency | If spec mentions threading, test race conditions |

#### Questions to Raise

- "This test asserts implementation detail X. Is X part of the spec?"
- "If we refactor the internals, would this test break incorrectly?"
- "What BDD scenario does this test correspond to?"
- "This test doesn't check the error type, only that an error occurred."

---

### 4. OSS Readiness (OSS公開適合性)

Ensure code is safe for public repository.

#### Security Checks

| Check | Pattern to Detect |
|-------|-------------------|
| Hardcoded secrets | API keys, tokens, passwords in code |
| Internal URLs | Internal hostnames, private IPs (10.x, 192.168.x, 172.16-31.x) |
| Personal information | Email addresses, names, phone numbers |
| Debug credentials | Test accounts with real-looking credentials |
| Unsafe crypto | Hardcoded keys, weak algorithms (MD5, SHA1 for security) |

#### Code Quality Checks

| Check | Issue |
|-------|-------|
| TODO with internal references | "TODO: Ask John" or internal ticket numbers |
| Comments with internal context | References to internal systems |
| Commented-out code blocks | Dead code that shouldn't be committed |
| Debug println!/dbg! macros | Should use proper logging |
| Hardcoded localhost/ports | Should be configurable |

#### License Compatibility

- Flag new dependencies without checking license
- Warn about GPL dependencies in MIT project

---

### 5. Guideline Compliance (ガイドライン準拠)

Check adherence to CLAUDE.md and project conventions.

#### Code Style

| Check | Requirement |
|-------|-------------|
| Comments language | English only in source code |
| Commit message | English, imperative mood, no prefix |
| Error handling | Use `thiserror`/`anyhow` patterns |
| Logging | Use `tracing` macros, not println! |

#### Architecture Rules

| Check | Requirement |
|-------|-------------|
| Real-time thread safety | No allocations in audio callback |
| Module boundaries | Respect API boundaries in docs-spec |
| Dependency direction | No circular dependencies |

#### Documentation Sync

If code changes affect:
- Public API → Check if `docs-spec/api/*.md` needs update
- Module structure → Check if `docs-spec/architecture.md` needs update
- Design decisions → Check if new ADR is needed

---

## Output Format

```
# Code Review: [target]

## Summary
- Files reviewed: N
- Issues found: N (Critical: X, Warning: Y, Info: Z)

---

## [filename]

### Critical Issues

#### [Issue Title]
- **Perspective**: Root Cause / Spec Compliance / OSS Readiness / Guidelines
- **Location**: Line X-Y
- **Problem**: Description of the issue
- **Current code**:
  ```rust
  problematic code
  ```
- **Recommendation**: What should be done instead
- **Suggested fix**:
  ```rust
  improved code
  ```

### Warnings

#### [Warning Title]
- **Perspective**: ...
- **Location**: Line X
- **Problem**: ...
- **Recommendation**: ...

### Info

- [Line X] Minor suggestion: ...

---

## Spec Sync Required

If implementation changes require spec updates:

| File | Update Needed |
|------|---------------|
| docs-spec/api/xxx.md | Add new function signature |
| docs-spec/adr/ | New ADR for design change |

---

## Action Items

1. [ ] Critical: Fix security issue in auth.rs
2. [ ] Warning: Update spec for new parameter
3. [ ] Info: Consider refactoring duplicate code
```

---

## Severity Levels

| Level | Criteria | Action |
|-------|----------|--------|
| Critical | Security risk, spec violation, broken functionality | Must fix before commit |
| Warning | Code smell, missing docs, potential issues | Should fix |
| Info | Style, minor improvements | Consider fixing |

---

## Example Review

```
# Code Review: git diff --staged

## Summary
- Files reviewed: 2
- Issues found: 3 (Critical: 1, Warning: 1, Info: 1)

---

## src/network/connection.rs

### Critical Issues

#### Hardcoded API Key
- **Perspective**: OSS Readiness
- **Location**: Line 45
- **Problem**: API key hardcoded in source
- **Current code**:
  ```rust
  let api_key = "sk-1234567890abcdef";
  ```
- **Recommendation**: Use environment variable or config file
- **Suggested fix**:
  ```rust
  let api_key = std::env::var("API_KEY")
      .expect("API_KEY environment variable required");
  ```

### Warnings

#### Symptomatic Fix
- **Perspective**: Root Cause Analysis
- **Location**: Line 78-82
- **Problem**: Adding sleep to fix race condition
- **Current code**:
  ```rust
  tokio::time::sleep(Duration::from_millis(100)).await;
  self.connect().await?;
  ```
- **Recommendation**: Investigate why connection needs delay. Consider proper synchronization or retry with backoff.

---

## src/audio/engine.rs

### Info

- [Line 23] Consider extracting duplicate buffer initialization into helper function

---

## Action Items

1. [x] Critical: Remove hardcoded API key
2. [ ] Warning: Investigate connection race condition
3. [ ] Info: Refactor buffer initialization
```
