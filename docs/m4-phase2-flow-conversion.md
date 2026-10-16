# M4 Phase 2: Flow-to-Mock Conversion

**Status:** ✅ Complete  
**Date:** 2026-10-17 (Designed: 2026-10-16, Implemented: 2026-10-17)

## Overview

Phase 2 will add the ability to convert captured HTTP request/response flows into reusable mock rules, enabling "record and replay" workflows.

## Goals

1. ✅ Convert captured flows to mock rules
2. ✅ Extract response bodies as fixture files
3. ✅ Generate appropriate match patterns from requests
4. ✅ Handle edge cases (large bodies, binary content, query params)
5. ✅ Provide control API for conversion

## Planned Features

### Flow Converter Module (`netproxyd/src/flow_converter.rs`)

```rust
pub struct ConvertOptions {
    pub fixtures_dir: PathBuf,
    pub inline_threshold: usize,  // Bodies smaller than this are inlined
    pub priority: i32,
    pub enabled: bool,
    pub match_query: bool,         // Include query params in match spec
    pub match_headers: Vec<String>, // Headers to match on
}

pub struct ConversionResult {
    pub rule: Rule,
    pub fixtures: Vec<Fixture>,    // Files to write
    pub warnings: Vec<String>,
}

pub struct Fixture {
    pub path: String,              // Relative to fixtures_dir
    pub content: Vec<u8>,
    pub content_type: Option<String>,
    pub size: usize,
}

pub fn convert_flow(
    reader: &Reader,
    run: &str,
    flow_id: u64,
    options: &ConvertOptions,
) -> Result<ConversionResult, ConvertError>;
```

### Control API - 2 New Methods

#### `mocks.from_flow` - Convert flow to rule (preview)

```json
{
  "method": "mocks.from_flow",
  "params": {
    "run": "current",           // or specific run ID
    "flow": 12345,
    "priority": 100,             // optional, default 100
    "enabled": true,             // optional, default true
    "inline_threshold": 1024,    // optional, default 1KB
    "match_query": false,        // optional, include query params
    "match_headers": ["authorization"], // optional
    "write_fixtures": false      // optional, write fixtures to disk
  }
}
```

**Response:**
```json
{
  "rule": {
    "id": "flow-abc123-12345",
    "priority": 100,
    "enabled": true,
    "description": "Converted from captured flow: GET api.example.com /users",
    "match": {
      "method": "GET",
      "host": "api.example.com",
      "path": "/users"
    },
    "action": {
      "type": "respond",
      "status": 200,
      "headers": {
        "content-type": "application/json"
      },
      "body": {
        "file": "flow-abc123-12345.json"
      }
    }
  },
  "fixtures": [
    {
      "path": "flow-abc123-12345.json",
      "size": 4567,
      "content_type": "application/json"
    }
  ],
  "warnings": [],
  "fixtures_written": false
}
```

#### `mocks.from_flow_and_add` - Convert and add atomically

```json
{
  "method": "mocks.from_flow_and_add",
  "params": {
    "run": "current",
    "flow": 12345,
    "priority": 100
  }
}
```

Converts the flow, writes fixtures, validates, and adds to dynamic registry in one operation.

## Implementation Strategy

### 1. Body Handling

NetProxy stores bodies in files (`bodies/<run>/<flow>.req|res`). Small bodies (<4KB) are also available in `BodyEnd` event's `preview_b64` field.

**Approach:**
- For small bodies: Use preview from `BodyEnd` event
- For large bodies: Read from body files using `bodies::read_range()`
- Text bodies under threshold: Inline as string
- Binary/large bodies: Save as fixture files

### 2. Match Pattern Generation

**Basic (default):**
```yaml
match:
  method: GET
  host: api.example.com
  path: /users/profile
```

**With query params (`match_query: true`):**
```yaml
match:
  method: GET
  host: api.example.com
  path: /users/profile
  query:
    user_id: "12345"
    format: "json"
```

**With headers:**
```yaml
match:
  method: POST
  host: api.example.com
  path: /auth/login
  headers:
    authorization: "Bearer *"  # Wildcard for token
```

### 3. Fixture File Naming

Pattern: `<rule-id>.<ext>`

Extensions determined by Content-Type:
- `application/json` → `.json`
- `text/html` → `.html`
- `image/png` → `.png`
- Unknown → `.bin`

### 4. Header Filtering

Skip headers that shouldn't be mocked:
- Connection management: `connection`, `keep-alive`, `transfer-encoding`
- Caching: `date`, `age`, `expires`, `etag`, `last-modified`
- Vary headers: `vary`

Include only meaningful response headers:
- `content-type`
- `cache-control`
- `content-disposition`
- Custom application headers

### 5. Edge Cases

**Large bodies:**
- Response bodies >10MB: Warn but proceed
- Very large bodies: Consider streaming in future

**Binary content:**
- Images, PDFs, etc.: Always save as fixtures
- Base64 decode preview if available

**Chunked/compressed bodies:**
- Store decompressed version
- Remove `content-encoding` header

**Failed requests:**
- Convert to `fail` action instead of `respond`
- Preserve error type

## Workflow Examples

### Example 1: Basic Conversion

```bash
# Capture a request
curl -x http://127.0.0.1:8899 https://api.example.com/users

# List flows to find ID
echo '{"jsonrpc":"2.0","id":1,"method":"flows.list","params":{"run":"current"}}' | \
  nc -U ~/Library/Application\ Support/NetProxy/run/control.sock

# Convert to mock (preview)
echo '{"jsonrpc":"2.0","id":2,"method":"mocks.from_flow","params":{"flow":1}}' | \
  nc -U ~/Library/Application\ Support/NetProxy/run/control.sock

# Convert and add
echo '{"jsonrpc":"2.0","id":3,"method":"mocks.from_flow_and_add","params":{"flow":1}}' | \
  nc -U ~/Library/Application\ Support/NetProxy/run/control.sock
```

### Example 2: Match with Query Params

```bash
# Convert with query matching
echo '{"jsonrpc":"2.0","id":4,"method":"mocks.from_flow_and_add","params":{
  "flow":2,
  "match_query":true,
  "priority":200
}}' | nc -U ~/Library/Application\ Support/NetProxy/run/control.sock
```

### Example 3: Match with Auth Header

```bash
# Convert with authorization header matching
echo '{"jsonrpc":"2.0","id":5,"method":"mocks.from_flow_and_add","params":{
  "flow":3,
  "match_headers":["authorization"],
  "priority":150
}}' | nc -U ~/Library/Application\ Support/NetProxy/run/control.sock
```

## Technical Challenges

### 1. Event API Complexity

NetProxy's event model requires:
- Reading from multiple event types (`RequestHead`, `ResponseHead`, `BodyEnd`)
- Handling preview data vs. body files
- Dealing with missing/incomplete flows

**Solution:** Helper functions to extract request/response from event stream

### 2. Body Storage

Bodies are split between:
- Event preview (small bodies)
- File storage (large bodies)
- Potentially incomplete/truncated

**Solution:** Check `BodyEnd` for stored vs. total bytes, use preview when complete

### 3. Pattern Ambiguity

Dynamic paths need smart pattern generation:
- `/users/12345` could be `/users/*` or `/users/{id}`
- `/api/v1/posts/abc-def-123` - what to match?

**Solution (Phase 3):** Pattern suggestion with multiple options

## Future Enhancements (Phase 3+)

### Smart Pattern Extraction
- Detect numeric IDs, UUIDs, slugs
- Suggest multiple match patterns
- Allow user to choose specificity

### Batch Conversion
- Convert multiple flows at once
- Merge similar patterns
- Deduplicate responses

### Request Body Matching
- Match on POST/PUT request bodies
- Extract fixtures from request side too

### Template Variables
- Parameterize responses (e.g., timestamps)
- Dynamic content generation

## Dependencies

- `base64` - Decode preview data
- `serde_json` - Parse response bodies to detect type
- Access to `np_store::Reader` for event queries
- Access to `bodies` module for file reading

## Testing Plan

### Unit Tests
- Match spec generation
- Header filtering
- Extension detection
- Inline threshold logic

### Integration Tests
- Full flow conversion end-to-end
- Fixture file creation
- Add to registry and match

### Manual Tests
- Large response bodies (>1MB)
- Binary content (images, PDFs)
- Compressed responses
- Failed requests

## Timeline Estimate

- **API research:** 0.5 days (understand event/body storage)
- **Core converter:** 1 day
- **Control API integration:** 0.5 days
- **Testing:** 0.5 days
- **Documentation:** 0.5 days

**Total:** ~3 days for Phase 2 complete implementation

## Success Criteria

✅ Convert any captured flow to a valid mock rule  
✅ Handle text and binary bodies correctly  
✅ Generate appropriate match patterns  
✅ Write fixtures to disk atomically  
✅ Validate generated rules before adding  
✅ Provide clear warnings for edge cases  
✅ Integration tests passing  

## Related Work

- **M4 Phase 1:** ✅ Complete - Dynamic registry foundation
- **M4 Phase 3:** Pattern suggestion & smart matching
- **M4 Phase 4:** MCP integration for AI-assisted mocking

---

**Note:** This is a design document. Implementation pending due to API surface complexity. Phase 1's dynamic registry provides the foundation; Phase 2 builds the conversion layer on top.
