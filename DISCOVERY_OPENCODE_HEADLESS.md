# Discovery: OpenCode Headless Mode Already Exists

**Date:** 2026-03-17  
**Impact:** Critical — Simplifies Issues #18 and #13 dramatically

---

## Summary

OpenCode already provides `opencode serve` — a production-ready headless HTTP server with:
- Full REST API (OpenAPI 3.1 spec)
- Session management
- Message sending (sync + async)
- File operations
- Authentication support
- **Built-in Playwright integration** (referenced in docs)

**Source:** https://opencode.ai/docs/server/

---

## What This Changes

### Before Discovery

**Issue #18:** "Implement Headless OpenCode Execution API"
- Estimated: 2 weeks (80 hours)
- Scope: Build headless execution from scratch
- Components: Process spawning, IPC, state management, SDK design

**Issue #13:** "Implement Playwright Test Runner"
- Estimated: 2 weeks (80 hours)
- Dependency: Wait for Issue #18 to complete
- Risk: Custom headless implementation might be buggy

### After Discovery

**Issue #18:** "Integrate OpenCode Headless Server for Plugin Use"
- **New Estimate: 10-13 hours** (8x faster!)
- Scope: Spawn `opencode serve`, manage lifecycle, expose to plugins
- Components: Process management, SDK integration, API endpoints

**Issue #13:** "Implement Playwright Test Runner"
- **New Estimate: 11-14 hours** (simplified architecture)
- Dependency: Still requires Issue #18, but much faster to complete
- Risk: Minimal — using battle-tested OpenCode server

**Total Time Saved:** ~140 hours → ~25 hours (5.6x faster)

---

## OpenCode Server Capabilities

### Command
```bash
opencode serve [--port <number>] [--hostname <string>] [--cors <origin>]
```

**Defaults:**
- Port: `4096`
- Hostname: `127.0.0.1`
- Auth: Set `OPENCODE_SERVER_PASSWORD` env var

### Key API Endpoints

#### Sessions
```
POST   /session              Create new session
GET    /session/:id          Get session details
POST   /session/:id/message  Send message (sync)
POST   /session/:id/prompt_async  Send message (async)
POST   /session/:id/abort    Abort session
DELETE /session/:id          Delete session
```

#### Files
```
GET /find?pattern=<pat>      Search text in files
GET /find/file?query=<q>     Find files by name
GET /file/content?path=<p>   Read file content
```

#### Config & Status
```
GET /global/health           Server health check
GET /config                  Get config
GET /provider                List providers
```

### OpenAPI Spec
Available at: `http://localhost:4096/doc`

---

## Architecture Changes

### Regression Testing Plugin Flow

**OLD (Custom Headless):**
```
Plugin → Custom Headless API → Spawn OpenCode → IPC → Playwright → Results
         ↑ (Need to build)
```

**NEW (Use opencode serve):**
```
Plugin → opencode serve (HTTP) → OpenCode Sessions → Playwright → Results
         ↑ (Already exists!)
```

### Implementation Simplification

**Phase 1: Daemon Integration (Issue #18)**
1. Spawn `opencode serve` processes (reuse existing process management)
2. Allocate ports (reuse `PortAllocator`)
3. Health checks via `/global/health`
4. Track in `WorkspaceStatus`

**Phase 2: SDK Usage (Issue #18)**
1. Install `@opencode-ai/sdk` (already published)
2. Create thin wrapper for plugin use
3. No custom protocol design needed

**Phase 3: Test Runner (Issue #13)**
1. Get headless server info from daemon API
2. Use SDK to send Playwright test prompts
3. Parse OpenCode responses
4. Store results

**Phase 4: UI (Issue #14)**
- No changes (still needs test report display)

---

## Code Examples

### Spawning Headless Server (Rust)
```rust
// daemon/src/workspace.rs
pub async fn start_headless_server(&self, workspace_name: &str, port: u16) -> Result<()> {
    let password = generate_random_password();
    let opencode_bin = self.config.opencode_repo.join("dist/opencode");
    
    let child = tokio::process::Command::new(&opencode_bin)
        .args(["serve", "--port", &port.to_string(), "--hostname", "127.0.0.1"])
        .env("OPENCODE_SERVER_PASSWORD", &password)
        .env("OPENCODE_CONFIG_DIR", workspace_path.join(".opencode"))
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    
    let key = format!("headless:{}", workspace_name);
    self.supervisor.track(key, child).await;
    
    // Health check
    for _ in 0..10 {
        if self.check_health(port).await.is_ok() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    
    Err(AppError::CommandFailed("Headless server failed to start".into()))
}
```

### Using SDK (TypeScript)
```typescript
// uwu-regression-testing-plugin/src/headless-client.ts
import { createClient } from '@opencode-ai/sdk'

export class TestRunnerClient {
  private client: any
  
  constructor(baseURL: string, password: string) {
    this.client = createClient({
      baseURL,
      auth: { username: 'opencode', password }
    })
  }
  
  async runTests(testCommand: string): Promise<TestResult> {
    // Create session
    const session = await this.client.session.create()
    
    // Send test prompt
    const prompt = `Run Playwright tests: ${testCommand} --reporter=json
Parse results and return structured JSON with test names, status, duration.`
    
    const response = await this.client.session.message.create(session.id, {
      parts: [{ type: 'text', text: prompt }]
    })
    
    // Parse response
    return this.parseResults(response)
  }
}
```

---

## Benefits

### 1. **Dramatically Reduced Complexity**
- No custom protocol design
- No IPC implementation
- No SDK from scratch
- Reuse OpenCode's battle-tested server

### 2. **Faster Development**
- Issue #18: 80 hours → 10-13 hours
- Issue #13: 80 hours → 11-14 hours
- Total: 160 hours → 25 hours

### 3. **Better Reliability**
- OpenCode server is production-tested
- OpenAPI spec ensures correctness
- Built-in error handling
- Established patterns

### 4. **Future Extensibility**
- Other plugins can use same headless infrastructure
- Full OpenCode API available (not just test runner)
- Can expose server to external tools

---

## Risks & Mitigations

### Risk 1: Fork Compatibility
**Issue:** We use a forked version of OpenCode. Will `opencode serve` work?

**Mitigation:**
- Check `opencode/packages/opencode/src/cli/cmd/serve.ts` — it's 24 lines, very simple
- Server implementation in `opencode/packages/opencode/src/server/server.ts`
- If needed, apply minimal patches to fork

### Risk 2: Playwright Integration
**Issue:** Docs mention Playwright but didn't find code references.

**Mitigation:**
- Test with real Playwright project first
- Prompt engineering: explicitly request Playwright installation + JSON reporter
- Fallback: OpenCode can still run bash commands to execute tests

### Risk 3: Resource Consumption
**Issue:** Each workspace could spawn a headless server (port + process).

**Mitigation:**
- Make headless servers **optional** (on-demand for testing plugin only)
- Auto-stop after test completion
- Port range: 5000-5099 (100 concurrent headless instances max)

---

## Action Items

### Immediate
- [x] Update Issue #18 with new scope (integration, not build)
- [x] Update Issue #13 with simplified architecture
- [x] Document discovery in this file

### Next Steps
1. **Test OpenCode headless mode** (manual verification)
   - Spawn `opencode serve` from forked repo
   - Hit `/global/health` endpoint
   - Create session via API
   - Send message and verify response

2. **Verify SDK compatibility**
   - Install `@opencode-ai/sdk` in test project
   - Check version compatibility with fork
   - Test basic session creation + message sending

3. **Begin Issue #18 implementation** (after Issue #19 complete)
   - Start with daemon integration
   - Add headless server lifecycle to workspace management
   - Expose endpoints

---

## References

- **OpenCode Server Docs:** https://opencode.ai/docs/server/
- **OpenCode SDK Docs:** https://opencode.ai/docs/sdk/
- **OpenAPI Spec:** `http://localhost:4096/doc` (when server running)
- **Forked Source:** `/opencode/packages/opencode/src/cli/cmd/serve.ts`
- **Issue #18:** https://github.com/vidwadeseram/uwu-my-opencode/issues/18
- **Issue #13:** https://github.com/vidwadeseram/uwu-my-opencode/issues/13

---

## Conclusion

This discovery is a **major win** for the project:
- **5.6x faster development time** for critical path (Issues #18 + #13)
- **Lower risk** by using production-tested server
- **Better architecture** with standard HTTP/REST patterns
- **More extensible** for future plugins

**Recommendation:** Prioritize testing OpenCode headless mode ASAP to validate this approach, then proceed with revised implementation plans.
