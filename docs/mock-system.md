# NetProxy Mock System Guide

The NetProxy mock system allows you to intercept HTTP/HTTPS requests and return predefined responses without hitting real servers. This is useful for:

- **Frontend Development** - Work on UI without backend dependencies
- **Testing** - Simulate various API responses and error conditions
- **Demos** - Show features with consistent, controlled data
- **Offline Development** - Continue working without network access
- **API Exploration** - Prototype against APIs that don't exist yet

## Table of Contents

- [Quick Start](#quick-start)
- [Configuration File](#configuration-file)
- [Rule Structure](#rule-structure)
- [Pattern Matching](#pattern-matching)
- [Actions](#actions)
- [Fixtures](#fixtures)
- [Priority System](#priority-system)
- [Examples](#examples)
- [Troubleshooting](#troubleshooting)

## Quick Start

1. **Create a mocks configuration file** at `~/Library/Application Support/NetProxy/mocks.yaml`:

```yaml
version: 1
fixtures_dir: ./fixtures

rules:
  - id: hello-world
    priority: 100
    enabled: true
    match:
      host: "test.local"
      path: "/hello"
    action:
      type: respond
      status: 200
      body:
        inline: "Hello from NetProxy!"
```

2. **Restart netproxyd** to load the rules:

```bash
netproxyd --data-dir ~/Library/Application\ Support/NetProxy
```

3. **Test it** through the proxy:

```bash
curl -x http://127.0.0.1:8899 http://test.local/hello
# Response: Hello from NetProxy!
```

## Configuration File

The mock configuration file (`mocks.yaml`) must be placed in your NetProxy data directory.

### File Structure

```yaml
version: 1                    # Config version (always 1 for now)
fixtures_dir: ./fixtures      # Path to fixture files (relative to mocks.yaml)

rules:                        # Array of mock rules
  - id: rule-name            # Unique identifier
    priority: 100            # Evaluation priority (higher = first)
    enabled: true            # Enable/disable this rule
    description: "..."       # Optional description
    match:                   # Matching criteria
      # ... match spec
    action:                  # Action to take
      # ... action spec
```

### Global Settings

- **`version`** (required): Configuration schema version. Currently always `1`.
- **`fixtures_dir`** (required): Directory containing fixture files. Can be:
  - Relative to `mocks.yaml` location (e.g., `./fixtures`)
  - Absolute path (e.g., `/Users/username/fixtures`)

## Rule Structure

Each rule has the following fields:

```yaml
- id: unique-rule-id              # Required: Unique identifier
  priority: 100                   # Optional: Default 0
  enabled: true                   # Optional: Default true
  description: "Rule description" # Optional: For documentation
  match:                          # Required: Match criteria
    # See Pattern Matching section
  action:                         # Required: What to do on match
    # See Actions section
```

### Rule Fields

- **`id`** (required): Unique identifier for the rule. Used in `MockHit` events.
- **`priority`** (optional, default: `0`): Higher priority rules are evaluated first.
- **`enabled`** (optional, default: `true`): Set to `false` to disable without deleting.
- **`description`** (optional): Human-readable description for documentation.
- **`match`** (required): Criteria for matching requests.
- **`action`** (required): What to do when the rule matches.

## Pattern Matching

The `match` section defines criteria for matching incoming requests. All specified criteria must match for the rule to trigger.

### Match Specification

```yaml
match:
  method: GET                          # HTTP method (GET, POST, etc.)
  host: "api.example.com"             # Host name
  path: "/api/users"                  # URL path
  path_regex: '/api/users/\d+'        # Regex pattern for path
  query:                              # Query parameters
    api_key: "test123"
    filter: "active"
  headers:                            # Request headers
    authorization: "Bearer *"
    content-type: "application/json"
```

All fields are optional. If a field is omitted, any value will match.

### Method Matching

Match specific HTTP methods (case-insensitive):

```yaml
match:
  method: GET   # Matches GET requests only
```

Supported methods: `GET`, `POST`, `PUT`, `DELETE`, `PATCH`, `HEAD`, `OPTIONS`

### Host Matching

Match the request host with exact matches or wildcards:

```yaml
# Exact match
match:
  host: "api.example.com"

# Wildcard subdomain
match:
  host: "*.example.com"    # Matches api.example.com, staging.example.com, etc.

# Match all hosts
match:
  host: "*"
```

**Wildcard Rules:**
- `*` matches any host
- `*.example.com` matches any subdomain of example.com
- Matching is case-insensitive

### Path Matching

Match URL paths with exact matches or wildcards:

```yaml
# Exact match
match:
  path: "/api/users"

# Wildcard suffix
match:
  path: "/api/*"           # Matches /api/users, /api/posts, etc.

# Wildcard prefix
match:
  path: "*/download"       # Matches /files/download, /media/download, etc.

# Match all paths
match:
  path: "*"
```

### Path Regex Matching

Use regular expressions for complex path patterns:

```yaml
match:
  path_regex: '/api/users/\d+'        # Matches /api/users/123, /api/users/456
  # OR
  path_regex: '/v\d+/.*'              # Matches /v1/anything, /v2/something
```

**Note:** If both `path` and `path_regex` are specified, both must match.

### Query Parameter Matching

Match requests based on query parameters:

```yaml
match:
  query:
    api_key: "test123"        # Exact match
    filter: "*"               # Any value (parameter must exist)
```

Query parameters support wildcards in values:
- `api_key: "test123"` - Exact match
- `api_key: "*"` - Any value

### Header Matching

Match requests based on HTTP headers:

```yaml
match:
  headers:
    authorization: "Bearer *"           # Wildcard match
    content-type: "application/json"    # Exact match
```

Header matching is case-insensitive for header names.

### Combining Criteria

All specified criteria must match (AND logic):

```yaml
match:
  method: POST
  host: "api.example.com"
  path: "/api/users"
  headers:
    content-type: "application/json"
# This matches ONLY if ALL conditions are true
```

## Actions

When a rule matches, its action determines what happens.

### Action Types

NetProxy supports three action types:

#### 1. Respond

Return a mock response:

```yaml
action:
  type: respond
  status: 200                          # HTTP status code
  headers:                             # Response headers
    content-type: application/json
    cache-control: no-cache
  body:                                # Response body
    file: response.json                # From fixture file
    # OR
    inline: "Hello, world!"            # Inline content
  delay_ms: 50                         # Optional: Simulated latency (M3: not implemented)
```

**Fields:**
- **`status`** (optional, default: `200`): HTTP status code
- **`headers`** (optional): Key-value map of response headers
- **`body`** (required): Response body (see [Fixtures](#fixtures))
- **`delay_ms`** (optional): Milliseconds to delay response (not implemented in M3)

#### 2. Pass

Let the request pass through to the real server:

```yaml
action:
  type: pass
```

This is useful for:
- Selective mocking (mock some endpoints, pass others)
- Conditionally disabling mocks
- Default catch-all behavior

**Note:** Pass actions do NOT emit `MockHit` events.

#### 3. Fail

Simulate a connection failure:

```yaml
action:
  type: fail
  error: connection_refused    # Error description
```

**Current Behavior (M3):** Returns a `503 Service Unavailable` response with the error message.

**Future (M4):** Will simulate actual connection failures.

## Fixtures

Fixtures define the response body content.

### File-Based Fixtures

Load content from a file:

```yaml
body:
  file: users.json    # Relative to fixtures_dir
```

**Features:**
- Files are cached after first load
- SHA-256 hashing for integrity
- Supports any file type (JSON, HTML, XML, images, etc.)

**Example Structure:**
```
~/Library/Application Support/NetProxy/
  ├── mocks.yaml
  └── fixtures/
      ├── users.json
      ├── profile.json
      └── error.html
```

### Inline Fixtures

Embed content directly in the rule:

```yaml
body:
  inline: "Hello, world!"
```

Or multi-line:

```yaml
body:
  inline: |
    {
      "status": "ok",
      "message": "This is inline JSON"
    }
```

**Use Cases:**
- Simple text responses
- Small JSON payloads
- Error messages
- Quick prototyping

## Priority System

Rules are evaluated in priority order (highest first). When multiple rules match a request, the first matching rule wins.

### Priority Values

- Higher numbers = Higher priority = Evaluated first
- Default priority: `0`
- Range: Any integer (typically 0-1000)

### Example

```yaml
rules:
  # Evaluated FIRST (priority 200)
  - id: specific-user
    priority: 200
    match:
      path: "/api/users/123"
    action:
      type: respond
      body:
        inline: "Specific user 123"

  # Evaluated SECOND (priority 100)
  - id: all-users
    priority: 100
    match:
      path: "/api/users/*"
    action:
      type: respond
      body:
        inline: "Generic user response"

  # Evaluated LAST (priority 0, default)
  - id: catch-all
    match:
      host: "*"
    action:
      type: respond
      status: 404
      body:
        inline: "Not found"
```

**Request to `/api/users/123`:**
1. `specific-user` matches (priority 200) → Returns "Specific user 123"
2. `all-users` is never evaluated

**Request to `/api/users/456`:**
1. `specific-user` doesn't match
2. `all-users` matches (priority 100) → Returns "Generic user response"

### Best Practices

- **Specific before general**: Give higher priority to specific rules
- **Use meaningful ranges**: e.g., 900-1000 for overrides, 100-500 for normal rules, 0-50 for catch-alls
- **Document priorities**: Use descriptions to explain why certain rules have high priority

## Examples

### Mock a REST API

```yaml
version: 1
fixtures_dir: ./fixtures

rules:
  # List users
  - id: list-users
    priority: 100
    enabled: true
    match:
      method: GET
      host: "api.example.com"
      path: "/api/v1/users"
    action:
      type: respond
      status: 200
      headers:
        content-type: application/json
      body:
        file: users-list.json

  # Get specific user
  - id: get-user
    priority: 100
    match:
      method: GET
      host: "api.example.com"
      path_regex: '/api/v1/users/\d+'
    action:
      type: respond
      status: 200
      headers:
        content-type: application/json
      body:
        file: user-detail.json

  # Create user (simulate success)
  - id: create-user
    priority: 100
    match:
      method: POST
      host: "api.example.com"
      path: "/api/v1/users"
    action:
      type: respond
      status: 201
      headers:
        content-type: application/json
      body:
        inline: |
          {
            "id": 999,
            "status": "created",
            "message": "User created successfully"
          }
```

### Mock with Environment-Specific Rules

```yaml
version: 1
fixtures_dir: ./fixtures

rules:
  # Production API - pass through
  - id: production-passthrough
    priority: 200
    enabled: true
    match:
      host: "api.example.com"
    action:
      type: pass

  # Staging API - mock everything
  - id: staging-users
    priority: 100
    enabled: true
    match:
      host: "staging.example.com"
      path: "/api/*"
    action:
      type: respond
      status: 200
      body:
        inline: '{"env":"staging","mocked":true}'

  # Development API - detailed mocks
  - id: dev-users-list
    priority: 100
    enabled: true
    match:
      host: "dev.example.com"
      path: "/api/users"
    action:
      type: respond
      status: 200
      body:
        file: dev-users.json
```

### Simulate Error Conditions

```yaml
version: 1
fixtures_dir: ./fixtures

rules:
  # Simulate rate limiting
  - id: rate-limit-error
    priority: 200
    enabled: false  # Enable to test rate limiting
    match:
      host: "api.example.com"
      path: "/api/*"
    action:
      type: respond
      status: 429
      headers:
        retry-after: "60"
      body:
        inline: '{"error":"rate_limit_exceeded","retry_after":60}'

  # Simulate server error
  - id: server-error
    priority: 200
    enabled: false
    match:
      host: "api.example.com"
    action:
      type: respond
      status: 500
      body:
        inline: '{"error":"internal_server_error"}'

  # Simulate connection failure
  - id: connection-failure
    priority: 200
    enabled: false
    match:
      host: "api.example.com"
    action:
      type: fail
      error: connection_timeout
```

### Complex Matching

```yaml
version: 1
fixtures_dir: ./fixtures

rules:
  # Match specific API key in query params
  - id: api-key-test
    priority: 100
    match:
      host: "api.example.com"
      query:
        api_key: "test_key_12345"
    action:
      type: respond
      status: 200
      body:
        inline: '{"env":"test","authorized":true}'

  # Match Authorization header
  - id: bearer-auth
    priority: 100
    match:
      host: "api.example.com"
      headers:
        authorization: "Bearer test-token-*"
    action:
      type: respond
      status: 200
      body:
        file: authenticated-response.json

  # Match POST with JSON content type
  - id: json-post
    priority: 100
    match:
      method: POST
      host: "api.example.com"
      headers:
        content-type: "application/json"
    action:
      type: respond
      status: 201
      body:
        inline: '{"status":"created"}'
```

## Troubleshooting

### Rules Not Loading

**Problem:** Changes to `mocks.yaml` don't take effect.

**Solution:** 
- Restart `netproxyd` (hot reload not available in M3)
- Check logs for YAML parse errors
- Verify `mocks.yaml` is in the data directory

### Fixture Not Found

**Problem:** Error: `fixture not found: users.json`

**Solutions:**
1. Verify the file exists in `fixtures_dir`
2. Check file permissions (must be readable)
3. Verify `fixtures_dir` path is correct (relative to `mocks.yaml`)

### Wrong Rule Matching

**Problem:** Unexpected rule is matching.

**Solutions:**
1. Check rule priorities (higher priority = evaluated first)
2. Verify match criteria are specific enough
3. Use more specific patterns for higher-priority rules
4. Check the `MockHit` event to see which rule matched

### No MockHit Event

**Problem:** Request goes through but no `MockHit` event appears.

**Reasons:**
- Rule has `type: pass` (pass actions don't emit MockHit)
- Rule is disabled (`enabled: false`)
- No rule matched (request passed through normally)
- Rule matching failed (check criteria)

### Debugging Tips

1. **Start simple**: Begin with a catch-all rule to verify mocks are working:
   ```yaml
   - id: test-catchall
     priority: 1000
     match:
       host: "*"
     action:
       type: respond
       status: 200
       body:
         inline: "Mock is working!"
   ```

2. **Check events**: Use the NetProxy UI or control API to view `MockHit` events

3. **Use inline bodies**: Test with inline bodies first, then switch to fixtures

4. **Disable conflicting rules**: Temporarily disable other rules to isolate issues

5. **Check the logs**: `netproxyd` logs YAML parse errors and fixture loading issues

## Event Tracking

When a mock rule matches, NetProxy emits two events:

### MockHit Event

```json
{
  "kind": "mock_hit",
  "flow": 42,
  "rule": "api-users"
}
```

### ResponseHead Event

```json
{
  "kind": "response_head",
  "flow": 42,
  "status": 200,
  "headers": [...],
  "mocked": true  // Indicates this was a mock response
}
```

These events allow you to:
- Track which mocks are being used
- Verify mocks are working correctly
- Debug unexpected behavior
- Analyze mock usage patterns

## Advanced Topics

### Fixtures Best Practices

1. **Organize by feature**:
   ```
   fixtures/
     ├── users/
     │   ├── list.json
     │   └── detail.json
     ├── posts/
     │   └── feed.json
     └── errors/
         └── 404.json
   ```

2. **Use realistic data**: Fixtures should mirror production responses

3. **Keep fixtures DRY**: Reuse common fixtures across multiple rules

4. **Version fixtures**: Consider versioning for API version testing

### Performance Considerations

- Fixtures are cached after first load (fast subsequent access)
- SHA-256 hashing adds minimal overhead
- Pattern matching is optimized for common cases
- Priority ordering ensures efficient rule evaluation

### Security Notes

- Mock rules apply to ALL traffic through the proxy
- Sensitive data in fixtures is visible in the data directory
- Use `.gitignore` to exclude `mocks.yaml` and `fixtures/` from version control
- Consider separate mock configs for different environments

---

**Next Steps:**
- [Architecture Overview](architecture.md) - System design details
- [Control API Reference](control-api.md) - Programmatic access
- [Development Guide](development.md) - Contributing to NetProxy
