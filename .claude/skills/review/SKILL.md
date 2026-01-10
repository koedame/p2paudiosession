---
name: review
description: Review specification documents for compliance with project guidelines. Use when checking docs-spec/ for ambiguous expressions or missing required sections.
allowed-tools: Read, Glob, Grep
---

# Specification Document Reviewer

## Instructions

1. Determine the review target:
   - If argument is provided: Review that specific file
   - If no argument: Find all files in `docs-spec/` directory using Glob

2. Read each target file

3. Apply the checks below based on file type

4. Output results in this format:
   ```
   ## [filename]

   ### Issues Found
   - [Line X] Issue description
     - Current: "problematic text"
     - Suggested: "improved text"

   ### OK (if no issues)
   No issues found.
   ```

---

## Check Rules

### Common Checks (All Files)

#### Prohibited Ambiguous Words
Detect these vague expressions and suggest numeric alternatives:

| Prohibited | Suggestion |
|------------|------------|
| 高速 | Specify: "< X ms" or "X ops/sec" |
| 高品質 | Specify: "MOS > X" or "bitrate X kbps" |
| 低遅延 | Specify: "RTT < X ms" or "latency < X ms" |
| シンプル | Specify concrete constraints |
| 柔軟 | Specify supported options explicitly |
| 高い/低い (performance) | Use numeric thresholds |
| 最適化 | Specify before/after metrics |

#### Judgment Reason Check
Flag statements that lack "why" explanation:
- Decisions without Context/Reason
- Technology choices without justification

---

### architecture.md Specific

Required sections (flag if missing):
- [ ] 対応 OS (macOS / Windows / Linux)
- [ ] 使用言語 (e.g., C++20, Rust stable)
- [ ] 音声 I/O (CoreAudio / WASAPI / ALSA / PipeWire)
- [ ] ネットワーク方式 (e.g., WebRTC Native)
- [ ] codec (e.g., Opus 48kHz / 20ms)
- [ ] スレッドモデル
- [ ] リアルタイム制約 (with numeric targets)

---

### ADR Files (docs-spec/adr/*.md)

Required sections:
- [ ] `## Context` - Why was this decision needed?
- [ ] `## Decision` - What was decided?
- [ ] `## Consequences` - Trade-offs and implications

Flag if:
- Any section is missing
- Context lacks problem statement
- Decision lacks clear choice
- Consequences lacks pros/cons

---

### BDD Files (docs-spec/behavior/*.feature)

Required structure:
- [ ] `Feature:` declaration
- [ ] At least one `Scenario:`
- [ ] Each scenario has `Given`, `When`, `Then`

Flag if:
- `Then` clause uses unmeasurable terms (suggest defining in architecture.md)
- Missing `Given` conditions for environment setup

---

### API Spec Files (docs-spec/api/*.md)

Required for each API:
- [ ] API name
- [ ] 入力 (Input parameters)
- [ ] 出力 (Return value)
- [ ] スレッド制約 (Thread requirements)
- [ ] 呼び出しタイミング (When to call)

Flag if:
- Thread constraints are missing (critical for real-time audio)
- Input/output types are unspecified

---

## Example Output

```
## docs-spec/architecture.md

### Issues Found
- [Line 15] Ambiguous expression detected
  - Current: "低遅延な音声通信を実現"
  - Suggested: "片方向遅延 < 150ms (RTT < 50ms環境)"

- [Missing] スレッドモデル section not found
  - Suggested: Add section describing thread architecture

### Summary
- 2 issues found
- 1 required section missing
```
