# GitHub Issues & Milestones Summary

**Created:** 2026-03-17  
**Last Updated:** 2026-03-17 (Critical bug fix added: Issue #19)  
**Repository:** https://github.com/vidwadeseram/uwu-my-opencode

---

## Milestones Created (6 total)

| # | Title | Due Date | Issues | Subtasks |
|---|-------|----------|--------|----------|
| 10 | Milestone 0: Critical Bug Fixes | 2026-03-24 | 1 | TBD |
| 5 | Milestone 1: Infrastructure Hardening | 2026-04-07 | 4 | 125 |
| 6 | Milestone 2: Plugin System Architecture | 2026-04-28 | 4 | 78 |
| 7 | Milestone 3: Regression Testing Plugin | 2026-05-19 | 2 | 51 |
| 8 | Milestone 4: AI Context System | 2026-05-26 | 2 | 53 |
| 9 | Milestone 5: Documentation & Polish | 2026-06-02 | 1 | 26 |

**Total Issues:** 14  
**Total Subtasks:** 333+ (detailed subtasks in Issues #6-#18)

---

## ⚠️ CRITICAL: Milestone 0 — Bug Fixes (MUST COMPLETE FIRST)

**Due:** March 24, 2026 (1 week)  
**Goal:** Fix critical UX/architecture bugs before feature development

### Issues

- [#19 Fix Auto-Start Bug: Workspaces Should Not Auto-Launch OpenCode](https://github.com/vidwadeseram/uwu-my-opencode/issues/19) — **CRITICAL**
  - Labels: `bug`, `critical`, `ux`, `architecture`
  - Priority: **HIGHEST**, Effort: Medium (8-11 hours)
  - **Problem:** Workspaces auto-start on creation (wrong behavior)
  - **Solution:** Separate creation from lifecycle, add manual Start/Stop buttons
  - **Blocks:** Issue #8 (service hosting), Issue #18 (headless API)
  - **Implementation Phases:**
    1. Backend status enum update (1-2 hours)
    2. Backend lifecycle management (2-3 hours)
    3. API testing (1 hour)
    4. Frontend integration (3-4 hours)
    5. Documentation (30 minutes)

**WHY THIS IS CRITICAL:**
- Users expect control over when workspaces run
- Auto-starting consumes resources without consent
- Proper state management is prerequisite for Issues #8 and #18
- Fixes fundamental UX/architecture flaw

**MUST FIX BEFORE:** Any work on Milestones 1-5

---

## Milestone 1: Infrastructure Hardening
**Due:** April 7, 2026  
**Goal:** Bulletproof installation and service hosting  
**Subtasks:** 125 total

### Issues
- [#6 Rename /host-project to /host](https://github.com/vidwadeseram/uwu-my-opencode/issues/6) — **15 subtasks**
  - Labels: `refactor`, `breaking-change`
  - Priority: High, Effort: Low
  - **Quick win** — Simple 2-file change

- [#7 Standardize Installation Process](https://github.com/vidwadeseram/uwu-my-opencode/issues/7) — **43 subtasks**
  - Labels: `installation`, `docker`, `shell`, `reliability`
  - Priority: High, Effort: High
  - Unify Docker/native paths, add validation, PATH setup, fonts

- [#8 Implement tmux-based Service Hosting System](https://github.com/vidwadeseram/uwu-my-opencode/issues/8) — **42 subtasks**
  - Labels: `feature`, `tmux`, `port-management`, `service-hosting`
  - Priority: High, Effort: Very High
  - Core feature: tmux windows for services, port registry

- [#18 Implement Headless OpenCode Execution API](https://github.com/vidwadeseram/uwu-my-opencode/issues/18) — **25 subtasks**
  - Labels: `feature`, `architecture`, `plugin-system`, `api`
  - Priority: **CRITICAL**, Effort: Very High
  - **PREREQUISITE for testing plugin** — Programmatic OpenCode execution

---

## Milestone 2: Plugin System Architecture
**Due:** April 28, 2026  
**Goal:** Enable external plugins with UI/logic contributions  
**Subtasks:** 78 total

### Issues
- [#9 Design Plugin Manifest Schema](https://github.com/vidwadeseram/uwu-my-opencode/issues/9) — **12 subtasks**
  - Labels: `plugin-system`, `architecture`, `design`
  - Priority: High, Effort: Medium
  - Define `plugin.json` format

- [#10 Implement Plugin Loading System](https://github.com/vidwadeseram/uwu-my-opencode/issues/10) — **23 subtasks**
  - Labels: `plugin-system`, `architecture`, `core`
  - Priority: High, Effort: Very High
  - Discovery, validation, dynamic loading

- [#11 Implement UI Slot System](https://github.com/vidwadeseram/uwu-my-opencode/issues/11) — **22 subtasks**
  - Labels: `plugin-system`, `ui`, `frontend`
  - Priority: High, Effort: High
  - React component injection slots

- [#12 Create Plugin Repository Structure](https://github.com/vidwadeseram/uwu-my-opencode/issues/12) — **21 subtasks**
  - Labels: `plugin`, `regression-testing`, `setup`
  - Priority: High, Effort: Low
  - New private repo scaffolding for uwu-regression-testing-plugin

---

## Milestone 3: Regression Testing Plugin
**Due:** May 19, 2026  
**Goal:** First external plugin — Playwright testing  
**Subtasks:** 51 total

### Issues
- [#13 Implement Playwright Test Runner](https://github.com/vidwadeseram/uwu-my-opencode/issues/13) — **20 subtasks**
  - Labels: `plugin`, `regression-testing`, `playwright`
  - Priority: High, Effort: Very High
  - Headless Linux VM test execution
  - **Depends on:** Issue #18 (Headless OpenCode API)

- [#14 Build Test Report UI](https://github.com/vidwadeseram/uwu-my-opencode/issues/14) — **31 subtasks**
  - Labels: `plugin`, `ui`, `regression-testing`
  - Priority: Medium, Effort: Medium
  - Toolbar button + sidebar panel

---

## Milestone 4: AI Context System
**Due:** May 26, 2026  
**Goal:** Folder-level AI context without manual prompting  
**Subtasks:** 53 total

### Issues
- [#15 Design AGENTS.md Hierarchy System](https://github.com/vidwadeseram/uwu-my-opencode/issues/15) — **26 subtasks**
  - Labels: `context`, `ai`, `documentation`
  - Priority: Medium, Effort: Medium
  - Auto-generate per-directory context docs

- [#16 Integrate AGENTS.md into OpenCode Context](https://github.com/vidwadeseram/uwu-my-opencode/issues/16) — **27 subtasks**
  - Labels: `context`, `ai`
  - Priority: Medium, Effort: Low
  - Auto-inject into agent context

---

## Milestone 5: Documentation & Polish
**Due:** June 2, 2026  
**Goal:** Comprehensive documentation coverage  
**Subtasks:** 26 total

### Issues
- [#17 Update Main README](https://github.com/vidwadeseram/uwu-my-opencode/issues/17) — **26 subtasks**
  - Labels: `documentation`
  - Priority: Medium, Effort: Low
  - Cover all new features with diagrams

---

## Quick Links

- **View all milestones:** https://github.com/vidwadeseram/uwu-my-opencode/milestones
- **View all issues:** https://github.com/vidwadeseram/uwu-my-opencode/issues
- **Roadmap document:** [ROADMAP.md](./ROADMAP.md)

---

## Implementation Sequence (Recommended)

### Phase 1: Foundation (Weeks 1-3)
**Goal:** Stabilize infrastructure and define plugin architecture

1. **#6** — Rename /host-project to /host (15 subtasks, 2-3 days)
   - Quick win to establish workflow
   
2. **#7** — Standardize Installation Process (43 subtasks, 1.5 weeks)
   - CRITICAL: PATH setup, fonts, nvim config
   - Foundation for all future work
   
3. **#9** — Design Plugin Manifest Schema (12 subtasks, 3-4 days)
   - Required before any plugin work

### Phase 2: Core Systems (Weeks 4-8)
**Goal:** Implement plugin infrastructure and service hosting

4. **#18** — Headless OpenCode Execution API (25 subtasks, 2 weeks)
   - **CRITICAL PATH** — Blocks Issue #13
   - Programmatic OpenCode execution for testing plugin
   
5. **#10** — Plugin Loading System (23 subtasks, 1.5 weeks)
   - Depends on: #9
   - Core plugin infrastructure
   
6. **#11** — UI Slot System (22 subtasks, 1 week)
   - Depends on: #10
   - Plugin UI injection points
   
7. **#8** — Tmux Service Hosting (42 subtasks, 2 weeks)
   - Can run in parallel with #18-#11
   - Service lifecycle management

### Phase 3: Testing Plugin (Weeks 9-13)
**Goal:** Build first external plugin

8. **#12** — Plugin Repository Structure (21 subtasks, 3-4 days)
   - Depends on: #9, #10
   - uwu-regression-testing-plugin scaffolding
   
9. **#13** — Playwright Test Runner (20 subtasks, 2 weeks)
   - Depends on: #18 (CRITICAL)
   - Headless test execution via API
   
10. **#14** — Test Report UI (31 subtasks, 1 week)
    - Depends on: #11, #13
    - Toolbar button + results panel

### Phase 4: AI Context (Weeks 14-16)
**Goal:** Auto-generate folder context

11. **#15** — AGENTS.md Hierarchy System (26 subtasks, 1 week)
    - Per-directory context generation
    
12. **#16** — AGENTS.md Integration (27 subtasks, 3-4 days)
    - Depends on: #15
    - Auto-inject into OpenCode

### Phase 5: Polish (Week 17)
**Goal:** Complete documentation

13. **#17** — README Update (26 subtasks, 3-4 days)
    - After all features complete
    - Architecture diagrams and screenshots

---

## Critical Path Dependencies

```
#6 (rename)           ─┐
                       ├──> #7 (install) ──> [ALL]
#9 (manifest)         ─┤
                       └──> #10 (loading) ──> #11 (UI slots) ──> #14 (test UI)
                                           │
                                           └──> #12 (plugin repo) ──> #13 (test runner)
                                                                        ▲
#18 (headless API) ────────────────────────────────────────────────────┘
                                                                      CRITICAL

#8 (tmux hosting) ──> [Independent, parallel with above]

#15 (AGENTS.md) ──> #16 (integration) ──> #17 (README)
```

**BLOCKING ISSUE:** #18 must complete before #13 can start. All other work can proceed in parallel.

---

## Labels Created (23 total)

- `refactor` — Code refactoring without functional changes
- `breaking-change` — Changes that break backward compatibility
- `installation` — Installation process and setup
- `docker` — Docker-related changes
- `shell` — Shell scripts and bash
- `reliability` — Stability and error handling
- `feature` — New feature or request
- `tmux` — Tmux integration and management
- `port-management` — Port allocation and conflicts
- `service-hosting` — Service hosting and process management
- `plugin-system` — Plugin architecture and loading
- `architecture` — System design and architecture
- `design` — Design decisions and specifications
- `core` — Core system functionality
- `ui` — User interface changes
- `frontend` — Frontend code and components
- `plugin` — Plugin development
- `regression-testing` — Regression testing plugin
- `setup` — Project setup and scaffolding
- `playwright` — Playwright testing
- `context` — AI context and documentation
- `ai` — AI agent improvements
- `documentation` — Documentation updates

---

## Next Steps

### Ready to Start (No Dependencies)
- **Issue #6** — Rename /host-project to /host (15 subtasks)
  - Quick win to validate workflow
  - No dependencies, can start immediately
  
- **Issue #7** — Standardize Installation (43 subtasks)
  - Foundation for all future work
  - Add tmux/opencode to PATH, install fonts/nvim
  
- **Issue #9** — Design Plugin Manifest (12 subtasks)
  - Required before plugin system work

### Critical Path
1. Complete **#18** (Headless API) BEFORE starting **#13** (Test Runner)
2. Complete **#9** (Manifest) BEFORE starting **#10** (Plugin Loading)
3. Complete **#10** (Loading) BEFORE starting **#11** (UI Slots) and **#12** (Plugin Repo)
4. Complete **#11** and **#13** BEFORE starting **#14** (Test Report UI)

### Parallelizable Work
- **#8** (Tmux Hosting) can run independently alongside plugin system work
- **#15** (AGENTS.md) can start anytime, doesn't block other work

### Final Steps
- **#16** and **#17** should be last (documentation after features complete)

---

## Issue Details

Use GitHub CLI to view full details:

```bash
gh issue view <number>
```

Or browse online:
- **View all milestones:** https://github.com/vidwadeseram/uwu-my-opencode/milestones
- **View all issues:** https://github.com/vidwadeseram/uwu-my-opencode/issues
- **Roadmap document:** [ROADMAP.md](./ROADMAP.md)

---

## Architecture Correction Note

**Issue #18 (Headless OpenCode Execution API) was added after initial planning** to correct the architecture for the regression testing plugin.

**Original Plan (WRONG):**
- Plugin directly spawns OpenCode processes

**Corrected Architecture:**
- Plugin → Headless API → OpenCode → Results
- Centralized execution with proper lifecycle management
- Enables future plugin reuse of execution infrastructure

See [ARCHITECTURE_CORRECTION_SUMMARY.md](./ARCHITECTURE_CORRECTION_SUMMARY.md) for full details.

---

## Subtask Summary by Issue

| Issue | Title | Subtasks | Milestone |
|-------|-------|----------|-----------|
| #6 | Rename /host-project to /host | 15 | 1 |
| #7 | Standardize Installation Process | 43 | 1 |
| #8 | Implement tmux-based Service Hosting | 42 | 1 |
| #18 | Implement Headless OpenCode Execution API | 25 | 1 |
| #9 | Design Plugin Manifest Schema | 12 | 2 |
| #10 | Implement Plugin Loading System | 23 | 2 |
| #11 | Implement UI Slot System | 22 | 2 |
| #12 | Create Plugin Repository Structure | 21 | 2 |
| #13 | Implement Playwright Test Runner | 20 | 3 |
| #14 | Build Test Report UI | 31 | 3 |
| #15 | Design AGENTS.md Hierarchy System | 26 | 4 |
| #16 | Integrate AGENTS.md into OpenCode Context | 27 | 4 |
| #17 | Update Main README | 26 | 5 |
| | **TOTAL** | **333** | |
