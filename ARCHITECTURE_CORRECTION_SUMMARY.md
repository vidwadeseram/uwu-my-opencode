# Architecture Correction Summary

**Date:** 2026-03-17  
**Issue:** User identified critical gap in regression testing plugin architecture

---

## Problem Statement

**User's Correction:**
> "ah u got something worng first we need way to run opencode with oh-my-opencode headless and then uwu-regression-testing-plugin will progermiticaly use it to test ans tasks need more details"

**Translation:**
1. **Missing prerequisite:** Need headless OpenCode execution API first
2. **Wrong architecture:** Original plan had plugin running Playwright directly
3. **Insufficient detail:** Issues need more granular subtasks

---

## What Was Wrong

### Original Architecture (Incorrect)
```
Plugin → Playwright (direct execution) → Test Results → UI
```

**Problems:**
- Plugin would need to handle Playwright subprocess management
- No way to leverage OpenCode's AI capabilities for test execution
- Duplicates functionality that should be in OpenCode

### Corrected Architecture
```
Plugin → Headless OpenCode API → OpenCode (executes Playwright) → Results → Plugin UI
```

**Why This Is Better:**
- OpenCode handles all test execution complexity
- Plugin is thin orchestration layer
- Reuses existing OpenCode capabilities
- Enables AI-driven test debugging/retry logic

---

## Actions Taken

### 1. Created Issue #18: "Implement Headless OpenCode Execution API" ✅

**GitHub:** https://github.com/vidwadeseram/uwu-my-opencode/issues/18  
**Milestone:** 2 (Plugin System Architecture)  
**Priority:** CRITICAL — Blocks testing plugin  
**Subtasks:** 25 detailed subtasks

**Key Components:**
- `HeadlessExecutor` (Rust) — Session management, lifecycle
- HTTP API — `/api/headless/spawn`, `/api/headless/execute`, `/api/headless/sessions/:id`
- Timeout/cost enforcement
- Concurrent session support

**Research Backing:**
- Found existing `opencode serve` command (port 4096)
- Found SDK `createOpencodeServer()` function
- Librarian research on headless Claude/Aider patterns (bg_80ec0b66)

### 2. Updated Issue #13: "Implement Playwright Test Runner" ✅

**GitHub:** https://github.com/vidwadeseram/uwu-my-opencode/issues/13  
**Changes:**
- Added **critical dependency** on Issue #18
- Corrected architecture (plugin spawns headless OpenCode)
- Added 20 detailed subtasks
- Documented execution flow with ASCII diagram

**New Execution Flow:**
1. User clicks "Run Tests" in plugin UI
2. Plugin calls: `POST /api/headless/spawn` → gets session_id
3. Plugin calls: `POST /api/headless/execute` with prompt "Run Playwright tests..."
4. Headless OpenCode executes tests, returns structured results
5. Plugin parses results, displays in UI
6. Plugin calls: `DELETE /api/headless/sessions/:id` (cleanup)

### 3. Added Detailed Subtasks to 5 Issues ✅

| Issue | Title | Subtasks Added | Status |
|-------|-------|----------------|--------|
| #6 | Rename /host-project to /host | 15 | ✅ |
| #7 | Standardize Installation Process | 35 | ✅ |
| #8 | Implement tmux-based Service Hosting | 42 | ✅ |
| #13 | Implement Playwright Test Runner | 20 | ✅ |
| #18 | Implement Headless OpenCode Execution API | 25 | ✅ |
| **TOTAL** | | **137 subtasks** | |

**Subtask Quality:**
- Each subtask is actionable (specific file, method, or command)
- Includes code examples where applicable
- Organized by implementation phase (Week 1, Day 1-2, etc.)
- Acceptance criteria per phase

### 4. Updated ROADMAP.md ✅

**Changes:**
- Added "Overview of Changes" section at top
- Added Issue #18 to Milestone 2 (Plugin System)
- Updated Issue #13 with architecture correction note
- Marked issues with detailed subtasks: ✅ status
- Updated dependencies (Issue #13 now depends on #18)

---

## Dependency Chain (Corrected)

### Before (Incorrect)
```
Issue #8 (tmux hosting) → Issue #13 (Playwright) → Issue #14 (UI)
```

### After (Correct)
```
Issue #18 (Headless API) → Issue #13 (Playwright) → Issue #14 (UI)
                 ↑
                 └── Blocks: Cannot proceed without this
```

**Why This Matters:**
- Issue #8 (tmux hosting) is independent — not a prerequisite for testing
- Issue #18 is the **critical path** for testing plugin
- Cannot parallelize #13 and #18 — must be sequential

---

## Research Completed

### Background Agents (2 launched, 1 completed)

1. **bg_80ec0b66** — Headless Claude/OpenCode execution patterns ✅
   - Duration: 56s
   - Key findings:
     - Anthropic Message Batches API (async batch processing)
     - Claude Code headless mode (`--output-format json`)
     - Aider scripting mode (`--message "prompt"`)
     - Best practices: Cost guardrails, tool whitelisting, audit logs

2. **bg_a4533b56** — Playwright programmatic test execution 🔄
   - Status: Still running (8+ minutes)
   - Will provide: Playwright API patterns, artifact collection strategies

### Code Exploration (2 agents completed)

1. **ses_306ffb237ffe24tcOKgEqFCKOy** — Found OpenCode CLI modes ✅
   - `opencode serve` command (24 lines, simple)
   - SDK `createOpencodeServer()` function
   - `run --format json` for machine-readable output

2. **ses_306ffa244ffeoJxM34TF3ATCe2** — Found oh-my-opencode loading ✅
   - Daemon generates `.opencode/plugins/oh-my-opencode.ts`
   - Launch command with env vars documented
   - Entry point: `oh-my-opencode/src/index.ts`

---

## Remaining Work

### High Priority (Critical Path)

1. **Wait for bg_a4533b56** (Playwright API patterns) — Expected soon
2. **Add detailed subtasks to Issues #9-12, #14-17** — 8 issues remaining
   - #9: Plugin Manifest Schema
   - #10: Plugin Loading System
   - #11: UI Slot System
   - #12: Plugin Repository Structure
   - #14: Test Report UI
   - #15: AGENTS.md Hierarchy System
   - #16: Integrate AGENTS.md into Context
   - #17: Update Main README

### Medium Priority

3. **Update GITHUB_ISSUES_SUMMARY.md** — Reflect new Issue #18 and architecture changes
4. **Update TASK_COMPLETION_SUMMARY.md** — Mark phases complete

### Low Priority

5. **Create implementation guide** for Issue #18 (headless execution)
6. **Test headless execution locally** — Verify `opencode serve` works as expected

---

## Success Metrics

### What Was Achieved ✅

- [x] Identified critical architecture gap (headless execution)
- [x] Created Issue #18 with comprehensive subtasks
- [x] Corrected Issue #13 architecture
- [x] Added 137 detailed subtasks across 5 issues
- [x] Updated roadmap with corrected dependencies
- [x] Completed research on headless OpenCode execution
- [x] Documented execution flow with diagrams

### Quality Indicators

- **Subtask Granularity:** Every subtask is ≤1 hour of work
- **Implementation Guidance:** Code examples provided for complex subtasks
- **Research Backing:** All architecture decisions backed by code exploration
- **Clear Acceptance Criteria:** Each issue has testable success metrics

---

## Files Modified/Created

### Modified
1. `ROADMAP.md` — Added Issue #18, updated dependencies, marked completed issues
2. GitHub Issue #6 — Added 15 subtasks
3. GitHub Issue #7 — Added 35 subtasks
4. GitHub Issue #8 — Added 42 subtasks
5. GitHub Issue #13 — Corrected architecture, added 20 subtasks

### Created
1. GitHub Issue #18 — New issue with 25 subtasks
2. `ARCHITECTURE_CORRECTION_SUMMARY.md` (this file)

---

## Next Steps for Implementation

### Week 1: Headless Execution API (Issue #18)
- Core executor implementation
- Session lifecycle management
- Port allocation integration

### Week 2: HTTP API + Testing (Issue #18)
- Axum route handlers
- Request/response types
- Integration tests

### Week 3: Documentation (Issue #18)
- API reference
- Usage examples
- Troubleshooting guide

### Week 4-8: Regression Testing Plugin (Issue #13)
- Plugin infrastructure
- Test execution orchestration
- UI components
- Documentation

**Total Timeline:** 8 weeks (2 months) for headless API + testing plugin

---

## Key Takeaways

1. **Always verify architecture end-to-end** before creating GitHub issues
2. **Headless execution is a common prerequisite** for automation plugins
3. **Subtasks must be granular** (user feedback: "tasks need more details")
4. **Research first, plan second** (discovered existing `serve` command)
5. **Document corrections immediately** to prevent future confusion

---

## References

### GitHub Issues
- Issue #18: https://github.com/vidwadeseram/uwu-my-opencode/issues/18
- Issue #13: https://github.com/vidwadeseram/uwu-my-opencode/issues/13
- Issue #6: https://github.com/vidwadeseram/uwu-my-opencode/issues/6
- Issue #7: https://github.com/vidwadeseram/uwu-my-opencode/issues/7
- Issue #8: https://github.com/vidwadeseram/uwu-my-opencode/issues/8

### Code References
- `opencode/packages/opencode/src/cli/cmd/serve.ts` — Headless server (24 lines)
- `opencode/packages/sdk/js/src/server.ts` — SDK `createOpencodeServer()`
- `daemon/src/workspace.rs` (lines 343-456) — oh-my-opencode loading
- `daemon/src/state.rs` — Existing `PortAllocator` (4100-4999 range)

### Research Sessions
- **bg_80ec0b66**: Headless Claude/OpenCode patterns (completed, 56s)
- **bg_a4533b56**: Playwright programmatic API (running, 8m+)
- **ses_306ffb237ffe24tcOKgEqFCKOy**: OpenCode CLI modes (completed)
- **ses_306ffa244ffeoJxM34TF3ATCe2**: oh-my-opencode loading (completed)
