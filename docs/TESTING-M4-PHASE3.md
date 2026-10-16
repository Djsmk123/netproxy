# Testing Guide: M4 Phase 3 - Smart Pattern Extraction

**Date:** 2026-10-17  
**Status:** Complete

## Overview

This document describes how to test the smart pattern extraction feature that automatically detects dynamic segments in URL paths and suggests match patterns with different specificity levels.

## Unit Tests

All unit tests are located in `crates/netproxyd/src/pattern_detector.rs`.

### Running Unit Tests

```bash
cd /Users/smkwinner/Desktop/workspace/netproxy
cargo test --package netproxyd --lib pattern_detector
```

### Test Coverage

1. **Pattern Type Detection**
   - `detects_numeric_id` - Verifies numeric ID detection (e.g., `/users/12345`)
   - `detects_uuid` - Verifies UUID detection (e.g., `550e8400-e29b-41d4-a716-446655440000`)
   - `detects_date` - Verifies date detection (e.g., `2024-10-17`)
   - `detects_timestamp` - Verifies Unix timestamp detection (e.g., `1697558400`)
   - `detects_hex` - Verifies hexadecimal value detection (e.g., `a1b2c3d4ef`)
   - `detects_slug` - Verifies slug detection (e.g., `my-awesome-post-2024`)

2. **Suggestion Generation**
   - `generates_three_specificity_levels` - Verifies exactly 3 suggestions (exact, moderate, broad)
   - `handles_path_with_no_dynamic_segments` - Tests paths with only static segments
   - `handles_mixed_segments` - Tests complex paths with mixed static/dynamic segments
   - `exact_pattern_always_matches_original` - Verifies exact patterns match input

## Integration Testing

### Test with Control API

1. **Analyze a Path**

```bash
# Start the daemon
cargo run --bin netproxyd -- --data ./test-data

# In another terminal, connect and test
echo '{"jsonrpc":"2.0","id":1,"method":"mocks.analyze_path","params":{"path":"/api/v1/users/12345/posts/550e8400-e29b-41d4-a716-446655440000"}}' | nc -U ./test-data/run/control.sock
```

**Expected Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "original": "/api/v1/users/12345/posts/550e8400-e29b-41d4-a716-446655440000",
    "segments": [
      {"type": "static", "value": "api"},
      {"type": "static", "value": "v1"},
      {"type": "static", "value": "users"},
      {"type": "numeric_id"},
      {"type": "static", "value": "posts"},
      {"type": "uuid"}
    ],
    "suggestions": [
      {
        "specificity": "exact",
        "pattern": "/api/v1/users/12345/posts/550e8400-e29b-41d4-a716-446655440000",
        "description": "Match this exact path only",
        "example_matches": [...]
      },
      {
        "specificity": "moderate",
        "pattern": "/api/v1/users/*/posts/*",
        "description": "Replace IDs/UUIDs with wildcards",
        "example_matches": [...]
      },
      {
        "specificity": "broad",
        "pattern": "/api/v1/*",
        "description": "Match all requests under this path",
        "example_matches": [...]
      }
    ]
  }
}
```

2. **Convert Flow with Pattern Analysis**

```bash
# Assuming you have captured a flow with ID 1
echo '{"jsonrpc":"2.0","id":2,"method":"mocks.from_flow","params":{"flow":1,"pattern_analysis":true}}' | nc -U ./test-data/run/control.sock
```

**Expected Response:**
Should include a `pattern_analysis` field in the result with the same structure as above.

3. **Convert Flow with Auto-Applied Pattern**

```bash
# Apply moderate specificity automatically
echo '{"jsonrpc":"2.0","id":3,"method":"mocks.from_flow","params":{"flow":1,"pattern_analysis":true,"pattern_specificity":"moderate"}}' | nc -U ./test-data/run/control.sock
```

**Expected Behavior:**
The generated rule's `path` field should use the moderate pattern (e.g., `/api/users/*/posts/*`) instead of the exact path.

## Manual Test Cases

### Test Case 1: User Profile Endpoint

**Input:** `/api/v1/users/12345/profile`

**Expected Output:**
- **Exact:** `/api/v1/users/12345/profile`
- **Moderate:** `/api/v1/users/*/profile`
- **Broad:** `/api/v1/*`

### Test Case 2: Order Details with UUID

**Input:** `/orders/550e8400-e29b-41d4-a716-446655440000/items`

**Expected Output:**
- **Exact:** `/orders/550e8400-e29b-41d4-a716-446655440000/items`
- **Moderate:** `/orders/*/items`
- **Broad:** `/orders/*`

### Test Case 3: Date-based Blog Post

**Input:** `/blog/2024/10/17/my-awesome-post`

**Expected Output:**
- **Exact:** `/blog/2024/10/17/my-awesome-post`
- **Moderate:** `/blog/*/*/*/*` (all dates and slugs replaced)
- **Broad:** `/blog/*`

### Test Case 4: Static-only Path

**Input:** `/api/v1/users`

**Expected Output:**
- **Exact:** `/api/v1/users`
- **Broad:** `/api/v1/*`
- **Note:** No moderate suggestion since there are no dynamic segments

### Test Case 5: Hex Value

**Input:** `/objects/a1b2c3d4ef56`

**Expected Output:**
- **Exact:** `/objects/a1b2c3d4ef56`
- **Moderate:** `/objects/*`
- **Broad:** `/objects/*`

## Edge Cases

1. **Empty Path:** `/`
   - Should return exact match only

2. **Single Segment:** `/users`
   - Should handle gracefully with exact and broad patterns

3. **Very Long Path:** `/a/b/c/d/e/f/g/h/i/j`
   - Should process all segments correctly

4. **Mixed Case:** `/API/V1/Users/123`
   - Keywords are lowercase-matched, so "API" and "Users" may not be recognized as static

## Performance Testing

Run pattern analysis on 1000 different paths:

```bash
# Create test script
cat > test_performance.sh << 'EOF'
#!/bin/bash
for i in {1..1000}; do
  echo "{\"jsonrpc\":\"2.0\",\"id\":$i,\"method\":\"mocks.analyze_path\",\"params\":{\"path\":\"/api/v1/users/$i/profile\"}}"
done | nc -U ./test-data/run/control.sock
EOF

chmod +x test_performance.sh
./test_performance.sh
```

**Expected:** All requests should complete successfully with reasonable response times (< 10ms per request).

## Success Criteria

- ✅ All unit tests pass
- ✅ All segment types detected correctly (UUID, numeric ID, date, timestamp, hex, slug)
- ✅ Exactly 3 specificity levels generated for paths with dynamic segments
- ✅ Broad patterns stop at version indicators (v1, v2, etc.)
- ✅ Pattern specificity can be auto-applied to flow conversion
- ✅ API returns pattern_analysis when requested
- ✅ No compiler warnings
- ✅ Integration with mocks.from_flow works end-to-end

## Known Limitations

1. Pattern detection is based on regex and static keyword lists
2. Custom business logic patterns are not automatically detected
3. GraphQL/gRPC paths are treated as regular HTTP paths
4. No learning from multiple similar flows (deferred to future phases)

## Troubleshooting

### Issue: Pattern not detected correctly

**Solution:** Check if the segment matches one of the regex patterns in `pattern_detector.rs`. You may need to add new keywords to `STATIC_KEYWORDS` or adjust regex patterns.

### Issue: Broad pattern too specific

**Solution:** The broad pattern logic stops after version indicators. If your API structure is different, you may need to adjust `find_static_prefix()`.

### Issue: Integration tests failing

**Solution:** The mock system integration tests may have environmental dependencies. Ensure you have a clean test data directory and the daemon is running properly.

## Next Steps

After confirming all tests pass, proceed to M4 Phase 4: MCP Integration.
