# uwu-my-opencode Roadmap & Milestones

**Generated:** 2026-03-17  
**Updated:** 2026-03-17 (Critical bug fix added: Issue #19, auto-start workspace bug)  
**Status:** Planning Phase — Detailed Subtasks Added

---

## Executive Summary

This document outlines the strategic roadmap for uwu-my-opencode, focusing on four core areas:

1. **Critical Bug Fixes** — Fix auto-start behavior, separate workspace creation from lifecycle management
2. **Infrastructure Refinement** — Standardize installation, rename hosting command, implement robust service management
3. **Plugin Architecture** — Enable external plugins with UI/logic contributions from separate repositories
4. **Quality Assurance** — Regression testing plugin with Playwright for automated end-to-end validation

---

## Overview of Changes (2026-03-17 Update)

**Critical Bug Fix Added:**
- **Added Milestone 0**: "Critical Bug Fixes" (due: March 24, 2026)
- **Added Issue #19**: "Fix Auto-Start Bug: Workspaces Should Not Auto-Launch OpenCode" — HIGH PRIORITY
  - Workspaces currently auto-start when created (wrong behavior)
  - Should have manual Start/Stop buttons with proper lifecycle management
  - Blocks other features: Issue #8 (service hosting) and #18 (headless API)

**Architecture Correction:**
- **Added Issue #18**: "Implement Headless OpenCode Execution API" — Prerequisite for testing plugin (Issue #13)
- **Updated Issue #13**: Corrected architecture — Plugin uses headless API → OpenCode executes tests → Plugin displays results
- **Added detailed subtasks** to all 13 issues (333+ actionable subtasks total)

**Why This Matters:**
Issue #19 is a critical UX bug that must be fixed first. Users expect control over when workspaces run, not automatic resource consumption on creation. The fix also establishes proper lifecycle state management needed for Issues #8 and #18.

---

## Milestone 0: Critical Bug Fixes

**Due:** March 24, 2026 (1 week)  
**Goal:** Fix critical UX/architecture bugs before new feature development

### Issues

#### Issue 0.1: Fix Auto-Start Bug [GitHub Issue #19]
**Status:** 🆕 Created  
**Priority:** **CRITICAL**  
**Effort:** Medium (8-11 hours)  
**Labels:** `bug`, `critical`, `ux`, `architecture`

**Problem:**
- Workspaces currently auto-start tmux + OpenCode + ttyd on creation
- Users have no control over when/if workspace processes run
- Resources immediately consumed without user consent

**Expected Behavior:**
- Create workspace → directory only, no processes
- Manual "Start" button → launches tmux + OpenCode + ttyd
- "Stop" button → kills all workspace processes
- "Running Projects" section shows active workspaces

**Root Cause:**
`ensure_workspace()` conflates creation and starting. No separation between:
- Workspace creation (directory + metadata)
- Workspace lifecycle (start/stop control)

**Acceptance Criteria:**
- [ ] Creating workspace does NOT start processes
- [ ] New workspaces have `status: Stopped`
- [ ] Workspace UI shows "Start" button when stopped
- [ ] Clicking "Start" launches tmux + OpenCode + ttyd
- [ ] Running workspaces show "Stop" button + terminal URL
- [ ] Status transitions work: `Stopped → Starting → Running → Stopping → Stopped`
- [ ] Multiple workspaces can run simultaneously
- [ ] Existing workspace data migrates correctly

**Blocks:**
- Issue #8 (tmux-based service hosting) — needs proper lifecycle
- Issue #18 (headless OpenCode API) — needs state management

**Implementation Estimate:** 8-11 hours (see issue for detailed phases)

**GitHub:** https://github.com/vidwadeseram/uwu-my-opencode/issues/19

---

## Milestone 1: Infrastructure Hardening

**Goal:** Bulletproof installation and service hosting.

### Issues

#### Issue 1.1: Rename `/host-project` to `/host` [GitHub Issue #6]
**Status:** ✅ Detailed subtasks added (15 subtasks)
**Priority:** High  
**Effort:** Low (2 file changes)  
**Labels:** `refactor`, `breaking-change`

**Description:**  
The `/host-project` command is verbose. Rename to `/host` for consistency.

**Acceptance Criteria:**
- [ ] Update `daemon/src/workspace.rs` (line 343, 365, 456)
- [ ] Update `README.md` (line 12)
- [ ] Verify no other references exist via `grep -r "host-project"`
- [ ] Test command execution in tmux workspace

**Technical Notes:**
- Only 2 files reference this command
- No directory named `/host-project` exists (command only)
- Backward compatibility: Could support both for 1 release

---

#### Issue 1.2: Standardize Installation Process [GitHub Issue #7]
**Status:** ✅ Detailed subtasks added (35 subtasks)  
**Priority:** High  
**Effort:** High  
**Labels:** `installation`, `docker`, `shell`, `reliability`

**Description:**  
Installation sometimes fails due to missing dependencies (opencode, nvim). Unify Docker and native installation paths with robust error handling and validation.

**Current State Analysis:**
- **Native Install** (`scripts/install.sh`): Installs GH CLI, Rust, builds tmux, runs Rust installer
- **Docker Install** (`scripts/install-docker.sh`): Installs Docker, clones repo, builds container
- **Rust Installer** (`daemon/src/installer.rs`): System packages, tmux, OpenCode deps, systemd, nginx, SSL
- **Inconsistencies:** Different error handling styles, missing dependency checks, no unified validation

**Acceptance Criteria:**
- [ ] Create unified dependency checker (`scripts/check-deps.sh`)
- [ ] Standardize error handling across all scripts (use `set -euo pipefail`)
- [ ] Add validation steps:
  - [ ] Verify tmux binary exists and is executable
  - [ ] Verify opencode installation with `opencode --version`
  - [ ] Verify nvim installation with `nvim --version`
- [ ] Create installation report (what succeeded, what failed, recovery steps)
- [ ] Document installation failure modes in troubleshooting guide
- [ ] Test on fresh Ubuntu 24.04, 22.04, and Debian 12

**Technical Approach:**
```bash
# scripts/check-deps.sh
check_command() {
  command -v "$1" &>/dev/null || {
    echo "ERROR: $1 not found" >&2
    return 1
  }
}

# Required tools
check_command gh
check_command cargo
check_command tmux
check_command opencode
check_command nvim
```

**Dependencies:** None

---

#### Issue 1.3: Implement tmux-based Service Hosting System [GitHub Issue #8]
**Status:** ✅ Detailed subtasks added (42 subtasks)  
**Priority:** High  
**Effort:** Very High  
**Labels:** `feature`, `tmux`, `port-management`, `service-hosting`

**Description:**  
When agents run web servers, frontends, or microservices, host them in dedicated tmux sessions so users can view logs and interact. Implement port allocation to prevent conflicts.

**Architecture Design:**

**Components:**
1. **ServiceRegistry** (Rust: `daemon/src/service_registry.rs`)
   - Tracks: `service_name` → `{tmux_session, tmux_window, port, pid, status}`
   - Persisted in state file
   - API: `register_service()`, `get_service()`, `list_services()`, `remove_service()`

2. **PortAllocator** (Rust: `daemon/src/port_allocator.rs`)
   - Dynamic port assignment from configured range (default: 4100-4999)
   - Checks availability with `TcpListener::bind()`
   - Thread-safe port reservation

3. **TmuxServiceManager** (Rust: `daemon/src/tmux_service_manager.rs`)
   - Creates dedicated tmux windows for services: `tmux new-window -t uwu-main -n "service-{name}"`
   - Injects environment variables (`PORT=8080`)
   - Monitors service health via pane PID checks
   - Cleanup on shutdown

4. **OpenCode Integration** (TypeScript: `oh-my-opencode/src/features/service-hosting/`)
   - Tool: `host_service` — Registers and launches services
   - Hook: Detects when agent starts a server, auto-registers
   - UI: Lists active services with URLs

**Port Management Strategy:**
```rust
// Allocate next available port in range
let port = port_allocator.allocate()?;

// Register service
service_registry.register(ServiceInfo {
    name: "frontend-dev",
    tmux_session: "uwu-main",
    tmux_window: "service-frontend-dev",
    port: port,
    pid: Some(process_id),
    status: ServiceStatus::Running,
});

// Launch in tmux
tmux_manager.spawn_service(
    "frontend-dev",
    vec!["PORT={}", "npm", "run", "dev"],
    port,
)?;
```

**Acceptance Criteria:**
- [ ] Port allocator finds available ports in configured range
- [ ] Service registry persists across daemon restarts
- [ ] Tmux windows created with naming convention: `service-{name}`
- [ ] Environment variable `PORT` injected correctly
- [ ] Service health monitoring detects crashes and updates status
- [ ] API endpoints:
  - [ ] `POST /workspace/{name}/service` — Start service
  - [ ] `GET /workspace/{name}/services` — List services
  - [ ] `DELETE /workspace/{name}/service/{id}` — Stop service
- [ ] OpenCode tool `host_service` callable by agents
- [ ] User can attach to service tmux window: `tmux select-window -t uwu-main:service-{name}`
- [ ] Documentation: How to view service logs, restart services, manage ports

**Technical Challenges:**
- **Port conflicts:** Handle case where allocated port is taken between check and bind
- **Zombie processes:** Ensure service cleanup when tmux window closes
- **Cross-process communication:** Daemon (Rust) ↔ OpenCode (TypeScript) via HTTP API

**Dependencies:** Issue 1.2 (installation must work first)

---

## Milestone 2: Plugin System Architecture

**Goal:** Enable external plugins in separate repos that contribute UI and logic.

### Issues

#### Issue 2.0: Implement Headless OpenCode Execution API [GitHub Issue #18] **[NEW]**
**Status:** ✅ Detailed subtasks added (25 subtasks)  
**Priority:** **CRITICAL** — Blocks regression testing plugin  
**Effort:** High (3 weeks)  
**Labels:** `plugin-system`, `core`, `architecture`, `feature`

**Description:**  
Create API for programmatically spawning and controlling OpenCode instances without GUI. This is a **prerequisite** for the regression testing plugin (Issue #13).

**Key Components:**
1. **HeadlessExecutor** (Rust) — Spawn/manage OpenCode server instances
2. **Session Management** — Track concurrent headless sessions
3. **HTTP API** — Endpoints for spawn/execute/close operations
4. **Lifecycle Management** — Graceful shutdown, timeout handling

**Critical Finding:**  
OpenCode already has `serve` command (port 4096) and SDK `createOpencodeServer()`. This issue wraps those primitives with production-ready lifecycle management.

**Why This Blocks Testing Plugin:**  
The testing plugin doesn't run Playwright directly — it spawns headless OpenCode, sends a "run tests" prompt, monitors execution, and displays results. Without this API, the plugin cannot function.

**Dependencies:** None (uses existing PortAllocator and OpenCode serve command)  
**Blocks:** Issue #13 (Playwright Test Runner)

---

#### Issue 2.1: Design Plugin Manifest Schema [GitHub Issue #9]
**Status:** ⏳ Needs detailed subtasks  
**Priority:** High  
**Effort:** Medium  
**Labels:** `plugin-system`, `architecture`, `design`

**Description:**  
Define the plugin manifest format (`plugin.json`) that plugins must provide.

**Schema Design:**
```json
{
  "name": "regression-testing-plugin",
  "version": "1.0.0",
  "displayName": "Regression Testing",
  "description": "Automated Playwright end-to-end testing for workspaces",
  "author": "your-name",
  "license": "MIT",
  "repository": "https://github.com/your-org/regression-testing-plugin",
  
  "entryPoint": "./dist/index.js",
  "type": "module",
  
  "contributes": {
    "ui": [
      {
        "slot": "workspace-toolbar",
        "component": "./dist/components/TestButton.js"
      }
    ],
    "tools": [
      {
        "name": "run_regression_tests",
        "handler": "./dist/tools/runTests.js"
      }
    ],
    "hooks": [
      {
        "event": "workspace.opened",
        "handler": "./dist/hooks/onWorkspaceOpen.js"
      }
    ],
    "mcps": [
      {
        "name": "playwright-mcp",
        "transport": "stdio",
        "command": "bun",
        "args": ["./dist/mcp/playwright-server.js"]
      }
    ]
  },
  
  "dependencies": {
    "@playwright/test": "^1.58.2"
  },
  
  "permissions": [
    "workspace:read",
    "workspace:execute",
    "tmux:spawn"
  ]
}
```

**Acceptance Criteria:**
- [ ] JSON schema definition with validation rules
- [ ] Documentation: Plugin Manifest Reference
- [ ] Example plugin scaffolding template
- [ ] Zod schema for TypeScript validation

**Dependencies:** None

---

#### Issue 2.2: Implement Plugin Loading System
**Priority:** High  
**Effort:** Very High  
**Labels:** `plugin-system`, `architecture`, `core`

**Description:**  
Extend OpenCode's plugin interface to discover, load, and manage external plugins from npm packages or git repos.

**Architecture:**

**Discovery:**
- Scan `~/.config/opencode/plugins/` for plugin directories
- Scan `.opencode/plugins/` for project-specific plugins
- Support npm package names (auto-install from registry)
- Support git URLs (clone to plugins directory)

**Loading Pipeline:**
```typescript
// oh-my-opencode/src/features/external-plugin-loader/

1. Discover plugins (scan directories + config)
2. Validate plugin.json against schema
3. Check permissions against allowed list
4. Dynamic import(pluginEntryPoint)
5. Register contributed components:
   - UI slots → React component registry
   - Tools → ToolRegistry
   - Hooks → HookRegistry
   - MCPs → SkillMcpManager
6. Initialize plugin with context API
```

**Context API:**
```typescript
interface PluginAPI {
  workspace: {
    getCurrent(): WorkspaceInfo;
    listFiles(pattern: string): Promise<string[]>;
    execute(command: string): Promise<ExecuteResult>;
  };
  tmux: {
    spawn(name: string, command: string[]): Promise<TmuxPane>;
    attach(paneId: string): Promise<void>;
  };
  ui: {
    registerComponent(slot: string, component: ReactComponent): void;
    showNotification(message: string, type: 'info' | 'error'): void;
  };
}
```

**Security:**
- Capabilities-based permission system
- Plugins declare required permissions in manifest
- User must approve on first load
- Sandboxing via `node:worker_threads` (not `isolated-vm` yet — overkill for MVP)

**Acceptance Criteria:**
- [ ] Plugin discovery from configured directories
- [ ] Manifest validation with helpful error messages
- [ ] Dynamic ESM import with error handling
- [ ] PluginAPI implementation with permission enforcement
- [ ] Plugin initialization lifecycle (init → ready → destroy)
- [ ] UI slot registry for React components
- [ ] Tool/Hook/MCP registration integration
- [ ] Error boundaries: Plugin crash doesn't kill OpenCode
- [ ] CLI command: `opencode plugins list`
- [ ] CLI command: `opencode plugins install <name|url>`
- [ ] Documentation: Plugin Development Guide

**Dependencies:** Issue 2.1

---

#### Issue 2.3: Implement UI Slot System
**Priority:** High  
**Effort:** High  
**Labels:** `plugin-system`, `ui`, `frontend`

**Description:**  
Create "slots" in the console UI where plugins can inject React components.

**Slot Locations:**
- `workspace-toolbar` — Buttons/actions in workspace header
- `sidebar-panel` — New sidebar tabs
- `status-bar` — Status indicators
- `context-menu` — Right-click menu items

**Implementation (SolidJS in console app):**
```typescript
// opencode/packages/console/app/src/components/PluginSlot.tsx

import { For } from "solid-js";
import { usePluginRegistry } from "../context/PluginContext";

export function PluginSlot(props: { name: string }) {
  const registry = usePluginRegistry();
  const components = () => registry.getComponents(props.name);
  
  return (
    <For each={components()}>
      {(Component) => <Component />}
    </For>
  );
}

// Usage in workspace layout:
<div class="workspace-toolbar">
  <PluginSlot name="workspace-toolbar" />
</div>
```

**Plugin Side:**
```typescript
// regression-testing-plugin/src/components/TestButton.tsx

export function TestButton() {
  const handleClick = async () => {
    await pluginAPI.tools.invoke("run_regression_tests");
  };
  
  return (
    <button onClick={handleClick}>
      Run Tests
    </button>
  );
}
```

**Acceptance Criteria:**
- [ ] PluginSlot component for each slot location
- [ ] Component registry with React 18 concurrent support
- [ ] Plugin components receive plugin API via context
- [ ] Error boundaries per plugin component
- [ ] Hot reload support during plugin development
- [ ] Documentation: UI Plugin Development

**Dependencies:** Issue 2.2

---

## Milestone 3: Regression Testing Plugin

**Goal:** First external plugin — automated Playwright testing.

### Issues

#### Issue 3.1: Create Plugin Repository Structure
**Priority:** High  
**Effort:** Low  
**Labels:** `plugin`, `regression-testing`, `setup`

**Description:**  
Create new private repository: `uwu-regression-testing-plugin`

**Repository Structure:**
```
uwu-regression-testing-plugin/
├── plugin.json
├── package.json
├── tsconfig.json
├── bun.lockb
├── README.md
├── src/
│   ├── index.ts                   # Plugin entry point
│   ├── components/
│   │   └── TestButton.tsx         # UI component
│   ├── tools/
│   │   └── runTests.ts            # Tool handler
│   ├── hooks/
│   │   └── onWorkspaceOpen.ts     # Hook handler
│   ├── mcp/
│   │   └── playwright-server.ts   # MCP server
│   └── playwright/
│       ├── config.ts              # Playwright config
│       └── tests/
│           └── app.spec.ts        # Test templates
├── dist/                          # Build output
└── .github/
    └── workflows/
        └── ci.yml
```

**Acceptance Criteria:**
- [ ] Repository created on GitHub (private)
- [ ] `plugin.json` with correct manifest
- [ ] TypeScript build pipeline (Bun)
- [ ] CI workflow for tests and build
- [ ] README with installation and usage instructions

**Dependencies:** Issue 2.1, Issue 2.2

---

#### Issue 3.2: Implement Playwright Test Runner [GitHub Issue #13]
**Status:** ✅ Detailed subtasks added (20 subtasks) + **Architecture corrected**  
**Priority:** High  
**Effort:** Very High  
**Labels:** `plugin`, `regression-testing`, `playwright`

**Architecture Correction:**  
**OLD (Incorrect):** Plugin runs Playwright directly  
**NEW (Correct):** Plugin → Headless OpenCode API (#18) → OpenCode executes Playwright → Plugin displays results

**Critical Dependency:**  
This issue **CANNOT proceed** without Issue #18 (Headless Execution API) being completed first.

**Description:**  
Core functionality: Run Playwright tests in headless Linux VM, report results.

**Architecture:**

**Components:**
1. **Test Discovery:** Scan workspace for `playwright.config.ts` or use plugin-provided config
2. **Tmux Integration:** Create dedicated tmux window for test execution
3. **Headless Setup:** Use `mcr.microsoft.com/playwright:v1.58.2-noble` Docker image patterns
4. **Reporter:** Custom reporter that captures screenshots, traces, videos
5. **Result Aggregation:** Parse Playwright JSON reporter output

**Implementation:**
```typescript
// src/tools/runTests.ts

export async function runTests(args: {
  workspace: string;
  testPath?: string;
  browser?: "chromium" | "firefox" | "webkit";
}) {
  // 1. Prepare test environment
  const tmuxWindow = await pluginAPI.tmux.spawn(
    "regression-tests",
    ["bash"]
  );
  
  // 2. Install Playwright with deps in tmux
  await tmuxWindow.execute("bun install");
  await tmuxWindow.execute("bunx playwright install --with-deps");
  
  // 3. Run tests with trace on failure
  const testCmd = [
    "bunx playwright test",
    args.testPath || "",
    `--browser=${args.browser || "chromium"}`,
    "--reporter=json,html",
  ].join(" ");
  
  const result = await tmuxWindow.execute(testCmd);
  
  // 4. Parse results
  const report = JSON.parse(result.stdout);
  const summary = generateSummary(report);
  
  // 5. Store artifacts
  await storeArtifacts(report, workspace);
  
  return summary;
}
```

**Headless Browser Setup (No GUI):**
- Use Playwright's built-in headless mode (no Xvfb needed)
- For headed debugging: Run `xvfb-run` wrapper in tmux
- Docker-based isolation (optional): Run tests in throwaway container

**Acceptance Criteria:**
- [ ] Tool callable via `pluginAPI.tools.invoke("run_regression_tests")`
- [ ] Tests run in dedicated tmux window
- [ ] Headless Chromium works on Linux VM
- [ ] Test report includes:
  - [ ] Pass/fail counts
  - [ ] Screenshots on failure
  - [ ] Trace files (`.zip`)
  - [ ] Video recordings
- [ ] Artifacts stored in `.uwu/test-reports/{workspace}/{timestamp}/`
- [ ] UI displays test results with links to artifacts
- [ ] Documentation: Writing Regression Tests

**Technical Notes:**
- Playwright Docker image: `mcr.microsoft.com/playwright:v1.58.2-noble`
- No Xvfb required (Playwright headless shell handles it)
- Trace capture: `trace: "on-first-retry"`
- Security: Run as non-root user in container

**Dependencies:** Issue 1.3 (tmux hosting), Issue 2.2 (plugin loading)

---

#### Issue 3.3: Build Test Report UI
**Priority:** Medium  
**Effort:** Medium  
**Labels:** `plugin`, `ui`, `regression-testing`

**Description:**  
Create UI component that displays test results in workspace toolbar and sidebar.

**UI Components:**
1. **Toolbar Button:** "Run Tests" — triggers test execution
2. **Status Indicator:** Shows running/passed/failed state
3. **Sidebar Panel:** Full test report with:
   - Summary stats (X passed, Y failed)
   - List of failed tests with links to traces
   - Screenshot thumbnails
   - "View in tmux" link to test window

**Implementation:**
```typescript
// src/components/TestButton.tsx

export function TestButton() {
  const [running, setRunning] = createSignal(false);
  const [report, setReport] = createSignal<TestReport | null>(null);
  
  const runTests = async () => {
    setRunning(true);
    try {
      const result = await pluginAPI.tools.invoke("run_regression_tests", {
        workspace: pluginAPI.workspace.getCurrent().name,
      });
      setReport(result);
      pluginAPI.ui.showNotification("Tests completed", "info");
    } catch (error) {
      pluginAPI.ui.showNotification(`Tests failed: ${error}`, "error");
    } finally {
      setRunning(false);
    }
  };
  
  return (
    <button onClick={runTests} disabled={running()}>
      {running() ? "Running..." : "Run Tests"}
    </button>
  );
}
```

**Acceptance Criteria:**
- [ ] Button in workspace toolbar
- [ ] Loading state during test execution
- [ ] Success/failure notification
- [ ] Sidebar panel with detailed report
- [ ] Clickable links to trace files
- [ ] Screenshot thumbnails
- [ ] Link to tmux window for live logs

**Dependencies:** Issue 2.3 (UI slots), Issue 3.2 (test runner)

---

## Milestone 4: AI Context System

**Goal:** Give AI agents folder-level context without manual prompting.

### Issues

#### Issue 4.1: Design AGENTS.md Hierarchy System
**Priority:** Medium  
**Effort:** Medium  
**Labels:** `context`, `ai`, `documentation`

**Description:**  
Similar to oh-my-opencode's `/init-deep`, auto-generate `AGENTS.md` files in each project directory explaining purpose, structure, and conventions.

**Hierarchy Example:**
```
project/
├── AGENTS.md              # Project overview
├── src/
│   ├── AGENTS.md          # Source code overview
│   ├── components/
│   │   └── AGENTS.md      # Component architecture
│   └── utils/
│       └── AGENTS.md      # Utility functions reference
└── tests/
    └── AGENTS.md          # Testing approach
```

**Auto-Generation Strategy:**
- Analyze directory structure
- Detect framework (React, Vue, Next.js, etc.)
- Scan for README files and extract relevant sections
- Generate AGENTS.md with:
  - **Purpose:** What this directory contains
  - **Structure:** File organization
  - **Conventions:** Naming, patterns, imports
  - **Key Files:** Most important files to read first

**Acceptance Criteria:**
- [ ] Command: `/generate-agents-md` (or integrate with `/init-deep`)
- [ ] Recursive generation for all directories with 2+ files
- [ ] Framework detection (package.json analysis)
- [ ] Template system for different project types
- [ ] Update mechanism (re-generate when structure changes)
- [ ] Hook: Auto-update AGENTS.md on major refactors

**Technical Approach:**
```typescript
// oh-my-opencode/src/features/agents-md-generator/

export async function generateAgentsMd(directory: string) {
  const files = await fs.readdir(directory);
  const structure = analyzeStructure(files);
  const framework = detectFramework(directory);
  
  const template = selectTemplate(framework, structure);
  const content = await renderTemplate(template, {
    files,
    structure,
    conventions: extractConventions(directory),
  });
  
  await fs.writeFile(
    path.join(directory, "AGENTS.md"),
    content
  );
}
```

**Dependencies:** None

---

#### Issue 4.2: Integrate AGENTS.md into OpenCode Context
**Priority:** Medium  
**Effort:** Low  
**Labels:** `context`, `ai`, `integration`

**Description:**  
Automatically inject relevant AGENTS.md files into agent context based on file locations.

**Strategy:**
- When agent reads a file: inject AGENTS.md from that directory + parent directories
- When agent asks "where is X": search all AGENTS.md files first
- Prevent context bloat: Use hierarchical injection (project → module → specific)

**Acceptance Criteria:**
- [ ] Hook: Inject AGENTS.md on file read
- [ ] Context prioritization: Specific > Module > Project
- [ ] Search integration: Query AGENTS.md before full codebase search
- [ ] Token budget awareness: Only inject if context allows

**Dependencies:** Issue 4.1

---

## Milestone 5: Documentation & Polish

### Issues

#### Issue 5.1: Update Main README
**Priority:** Medium  
**Effort:** Low  
**Labels:** `documentation`

**Description:**  
Comprehensive README update covering new features and systems.

**Sections to Add/Update:**
- [ ] **Service Hosting:** How to use tmux-based hosting
- [ ] **Port Management:** Understanding port allocation
- [ ] **Plugin System:** Installing and using plugins
- [ ] **Regression Testing Plugin:** End-to-end testing guide
- [ ] **AGENTS.md System:** Context management for AI
- [ ] **Troubleshooting:** Common issues and solutions

**Acceptance Criteria:**
- [ ] Clear installation instructions (both Docker and native)
- [ ] Architecture diagram of system components
- [ ] Screenshots of key features
- [ ] Links to detailed guides in `/docs`

**Dependencies:** All previous milestones

---

## Implementation Strategy

### Phase 0: Critical Bug Fixes (Week 1)
**MUST COMPLETE BEFORE NEW FEATURES**

1. **Issue #19** — Fix auto-start workspace bug (8-11 hours)
   - Update `WorkspaceStatus` enum (`Stopped | Starting | Running | Stopping`)
   - Separate creation from lifecycle management
   - Remove auto-start from `ensure_workspace()`
   - Add Start/Stop buttons to frontend
   - Migrate existing state files

**Rationale:** This bug blocks Issues #8 (service hosting) and #18 (headless API). Proper lifecycle management must exist before building on top of it.

---

### Phase 1: Foundation (Weeks 2-3)
**Prerequisites:** Phase 0 complete

1. Rename `/host-project` to `/host` (Issue #6) — 2-3 days
2. Standardize installation (Issue #7) — 1.5 weeks
3. Design plugin manifest schema (Issue #9) — 3-4 days

### Phase 2: Core Systems (Weeks 4-8)
1. **Issue #18** — Headless OpenCode Execution API (2 weeks) — **CRITICAL PATH**
2. Implement tmux service hosting (Issue #8) — 2 weeks (parallel with #18)
3. Build plugin loading system (Issue #10) — 1.5 weeks
4. Create UI slot system (Issue #11) — 1 week

### Phase 3: Testing Plugin (Weeks 9-13)
**Prerequisites:** Issue #18 complete

1. Set up plugin repository (Issue #12) — 3-4 days
2. Implement Playwright runner (Issue #13) — 2 weeks
3. Build test report UI (Issue #14) — 1 week

### Phase 4: Context System (Weeks 14-16)
1. Design AGENTS.md hierarchy (Issue #15) — 1 week
2. Integrate with OpenCode (Issue #16) — 3-4 days

### Phase 5: Documentation (Week 17)
1. Update README (Issue #17) — 3-4 days
2. Write migration guides
3. Create architecture diagrams

---

## Technical Debt & Considerations

### Critical Bug (MUST FIX FIRST)
**Issue #19:** Auto-start workspace behavior breaks user control. Creation should NOT start processes. Start/Stop must be manual user actions.

### Known Challenges
1. **Port Conflicts:** Race conditions between allocation and binding
2. **Tmux Stability:** Handling zombie processes and crashed panes
3. **Plugin Security:** Permission system must prevent privilege escalation
4. **Cross-Language Communication:** Rust daemon ↔ TypeScript OpenCode via HTTP
5. **Playwright in Docker:** Shared memory and seccomp profile issues
6. **Workspace Lifecycle:** Proper state management (Stopped → Running transitions)

### Future Enhancements
- **Plugin Marketplace:** Centralized plugin discovery
- **Hot Reload:** Plugin updates without restart
- **Distributed Testing:** Shard tests across multiple VMs
- **Visual Regression:** Screenshot diffing for UI changes
- **CI/CD Integration:** GitHub Actions workflow templates

---

## Success Metrics

| Metric | Target | Current |
|--------|--------|---------|
| Workspace Start/Stop Control | 100% user-controlled | ❌ Auto-starts |
| Installation Success Rate | >95% | TBD |
| Service Hosting Reliability | >99% uptime | TBD |
| Plugin Load Time | <500ms | TBD |
| Test Execution Time (100 tests) | <2 minutes | TBD |
| Documentation Coverage | 100% of features | TBD |

---

## Contributing

See individual issue templates for contribution guidelines. All issues will be tracked in GitHub with appropriate labels and milestones.

---

## Questions & Feedback

Open an issue or discussion in the main repository with the `roadmap` label.
