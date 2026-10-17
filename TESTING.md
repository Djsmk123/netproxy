# NetProxy Testing Guide

Quick guide to test NetProxy features, especially the new JSON Viewer.

## Quick Start

### 1. Start Fresh

```bash
# Clean start with all processes killed
./scripts/start-fresh.sh

# Daemon is now running in background
```

### 2. Run SwiftUI App

```bash
cd app
swift run
```

**Or open in Xcode:**
```bash
open app/Package.swift
```

### 3. Test JSON Viewer

1. **In the NetProxy app:**
   - Click "Start Recording" (⌘R)
   
2. **Generate JSON traffic** (choose one method):
   
   **Option A: Use your browser with system proxy**
   ```bash
   # Set system proxy (macOS)
   networksetup -setwebproxy Wi-Fi 127.0.0.1 8899
   networksetup -setsecurewebproxy Wi-Fi 127.0.0.1 8899
   
   # Visit JSON APIs in browser
   open https://httpbin.org/json
   open https://api.github.com/users/djsmk123
   
   # Clear system proxy when done
   networksetup -setwebproxystate Wi-Fi off
   networksetup -setsecurewebproxystate Wi-Fi off
   ```
   
   **Option B: Use curl with proxy**
   ```bash
   curl -x http://127.0.0.1:8899 -k https://httpbin.org/json
   curl -x http://127.0.0.1:8899 -k https://api.github.com/users/djsmk123
   curl -x http://127.0.0.1:8899 -k https://jsonplaceholder.typicode.com/posts/1
   ```
   
   **Option C: Use Chrome with proxy (via MCP)**
   ```bash
   # If you have MCP configured
   # Tell Claude: "Launch Chrome with NetProxy"
   ```

3. **View JSON in the app:**
   - Select a flow from the list
   - Click "Response" tab
   - Click "Body" sub-tab
   - **JSON Viewer appears with 4 modes!**

### JSON Viewer Features to Test

**Tree View (Default):**
- ✅ Expand/collapse nodes (click arrows)
- ✅ See syntax colors (green strings, blue numbers, purple booleans)
- ✅ Right-click for context menu
- ✅ Copy Path (⌘⌥C)
- ✅ Copy Value
- ✅ Copy JSONPointer
- ✅ Search with highlight

**Raw View:**
- ✅ Line numbers
- ✅ Syntax highlighting
- ✅ Format button
- ✅ Minify button
- ✅ Search

**Table View:**
- ✅ Auto-shows for arrays of objects
- ✅ Sortable columns
- ✅ Export to CSV

**Search:**
- ✅ Type in search bar
- ✅ Matches highlight in yellow
- ✅ Toggle Keys/Values filter

## Process Management

### Kill All NetProxy Processes

**Interactive (asks confirmation):**
```bash
./scripts/kill-netproxy.sh
```

**Force (no confirmation):**
```bash
./scripts/kill-netproxy-force.sh
```

### Check Running Processes

```bash
ps aux | grep netproxy
```

### View Daemon Logs

```bash
tail -f netproxy-daemon.log
```

## Testing Checklist

### Phase 1: JSON Viewer

- [ ] Tree view renders JSON hierarchically
- [ ] Expand/collapse works
- [ ] Syntax colors show correctly (dark mode)
- [ ] Copy path generates correct JSONPath
- [ ] Search highlights matches
- [ ] Raw view shows line numbers
- [ ] Format/Minify work
- [ ] Table view shows for arrays
- [ ] Invalid JSON shows error with line/column
- [ ] Large JSON (1MB+) loads without freezing

### MCP Integration

- [ ] `engine_start` starts proxy
- [ ] `engine_status` shows state
- [ ] `flows_list` shows captured flows
- [ ] `mocks.from_flow` converts flow to mock
- [ ] `system_proxy_set` configures macOS proxy
- [ ] `chrome_launch_with_proxy` opens Chrome

### Mock System (M4 Phases 1-3)

- [ ] Static rules load from `mocks.yaml`
- [ ] Dynamic rules can be added via API
- [ ] Hot reload works (edit YAML, rules reload)
- [ ] Validation detects issues
- [ ] Pattern detection suggests match options
- [ ] Flow-to-mock conversion works

## Common Issues

### "Another netproxyd is already running"

**Solution:**
```bash
./scripts/kill-netproxy-force.sh
./scripts/start-fresh.sh
```

### "Connection refused" on curl

**Problem:** Engine not started

**Solution:**
1. Check if daemon is running: `ps aux | grep netproxyd`
2. Start fresh: `./scripts/start-fresh.sh`
3. Start engine via app or API

### JSON Viewer not showing

**Possible causes:**
1. Content-Type is not `application/json`
2. Response body is empty
3. Inspector tab not selected

**Solution:**
1. Check response headers
2. Try different JSON endpoint
3. Select flow, click Response tab, then Body

## API Endpoints for Testing

**JSON APIs:**
- `https://httpbin.org/json` - Simple JSON object
- `https://api.github.com/users/djsmk123` - User profile
- `https://jsonplaceholder.typicode.com/posts` - Array of posts
- `https://jsonplaceholder.typicode.com/posts/1` - Single post
- `https://catfact.ninja/fact` - Random cat fact

**Large JSON:**
- `https://jsonplaceholder.typicode.com/posts` (5KB)
- `https://jsonplaceholder.typicode.com/comments` (30KB)

## Next Steps

After testing Phase 1 (JSON Viewer):

- **Phase 2:** Request/Response Inspector (Headers, Cookies, Timing)
- **Phase 3:** Enhanced Traffic List
- **Phase 4:** Mock Manager UI
- **Phase 5:** Devices Configuration
- **Phase 6:** Session Management

---

**Questions?** Check:
- Design docs: `/docs/ui-redesign/`
- Implementation plan: `/docs/ui-redesign/05-implementation-plan.md`
- Architecture: `/docs/architecture.md`
