# M4 Phase 3: Smart Pattern Extraction

**Status:** ✅ Complete  
**Date:** 2026-10-17  
**Completed:** 2026-10-17

## Overview

Automatically detect dynamic segments in URLs (UUIDs, IDs, timestamps) and suggest multiple match pattern options when converting flows to mocks, enabling flexible rule creation.

## Goals

1. Detect common dynamic patterns in paths:
   - Numeric IDs (`/users/12345` → `/users/*` or `/users/{id}`)
   - UUIDs (`/orders/550e8400-e29b-41d4-a716-446655440000` → `/orders/*`)
   - Slugs (`/posts/my-awesome-post-2024` → `/posts/*`)
   - Timestamps (`/logs/2024-10-17` → `/logs/*`)
   - Hex values (`/objects/a1b2c3d4` → `/objects/*`)

2. Suggest multiple pattern options with specificity levels:
   - **Exact match** - Match only this specific request
   - **Parameterized** - Replace dynamic segments with wildcards
   - **Generic** - Broad wildcard matching

3. Let user choose pattern specificity (API parameter or UI selection)

4. Optionally extract parameter names from path structure

## Non-Goals (Defer to Later)

- Request body pattern matching
- Learning patterns from multiple similar flows
- Pattern suggestion based on API documentation
- GraphQL/gRPC-specific pattern extraction

---

## Pattern Detection

### Pattern Types

```rust
#[derive(Debug, Clone, Serialize)]
pub enum PathSegmentType {
    Static(String),           // "users", "api", "v1"
    NumericId,                // "12345", "999"
    Uuid,                     // "550e8400-e29b-41d4-a716-446655440000"
    Slug,                     // "my-post-title"
    Date,                     // "2024-10-17"
    Timestamp,                // "1697558400"
    Hex,                      // "a1b2c3d4ef"
    Other(String),            // Unrecognized dynamic content
}

#[derive(Debug, Clone, Serialize)]
pub struct PathAnalysis {
    pub original: String,
    pub segments: Vec<PathSegmentType>,
    pub pattern_suggestions: Vec<PatternSuggestion>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PatternSuggestion {
    pub specificity: Specificity,
    pub pattern: String,
    pub description: String,
    pub example_matches: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum Specificity {
    Exact,        // Match only this request
    Moderate,     // Replace detected IDs/UUIDs
    Broad,        // Wildcard all dynamic segments
}
```

### Detection Logic

```rust
pub fn analyze_path(path: &str) -> PathAnalysis {
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    
    let analyzed_segments: Vec<PathSegmentType> = segments.iter()
        .map(|seg| classify_segment(seg))
        .collect();
    
    let suggestions = generate_suggestions(path, &analyzed_segments);
    
    PathAnalysis {
        original: path.to_string(),
        segments: analyzed_segments,
        pattern_suggestions: suggestions,
    }
}

fn classify_segment(segment: &str) -> PathSegmentType {
    // UUID: 8-4-4-4-12 hex pattern
    if UUID_REGEX.is_match(segment) {
        return PathSegmentType::Uuid;
    }
    
    // Numeric ID: all digits
    if segment.chars().all(|c| c.is_ascii_digit()) {
        return PathSegmentType::NumericId;
    }
    
    // Date: YYYY-MM-DD
    if DATE_REGEX.is_match(segment) {
        return PathSegmentType::Date;
    }
    
    // Unix timestamp: 10-digit number
    if segment.len() == 10 && segment.chars().all(|c| c.is_ascii_digit()) {
        return PathSegmentType::Timestamp;
    }
    
    // Hex: only a-f0-9
    if segment.len() >= 8 && segment.chars().all(|c| c.is_ascii_hexdigit()) {
        return PathSegmentType::Hex;
    }
    
    // Slug: contains hyphens, lowercase alphanumeric
    if SLUG_REGEX.is_match(segment) {
        return PathSegmentType::Slug;
    }
    
    // Static segment (API keywords, etc.)
    if is_static_keyword(segment) {
        return PathSegmentType::Static(segment.to_string());
    }
    
    // Unknown dynamic content
    PathSegmentType::Other(segment.to_string())
}
```

### Pattern Generation

```rust
fn generate_suggestions(path: &str, segments: &[PathSegmentType]) -> Vec<PatternSuggestion> {
    let mut suggestions = Vec::new();
    
    // 1. Exact match (lowest risk)
    suggestions.push(PatternSuggestion {
        specificity: Specificity::Exact,
        pattern: path.to_string(),
        description: "Match this exact path only".to_string(),
        example_matches: vec![path.to_string()],
    });
    
    // 2. Moderate - replace detected IDs/UUIDs
    let moderate_pattern = segments.iter()
        .map(|seg| match seg {
            PathSegmentType::NumericId => "*".to_string(),
            PathSegmentType::Uuid => "*".to_string(),
            PathSegmentType::Hex => "*".to_string(),
            PathSegmentType::Date => "*".to_string(),
            PathSegmentType::Timestamp => "*".to_string(),
            PathSegmentType::Static(s) => s.clone(),
            PathSegmentType::Slug => "*".to_string(),
            PathSegmentType::Other(s) => s.clone(),
        })
        .collect::<Vec<_>>()
        .join("/");
    
    if moderate_pattern != path {
        suggestions.push(PatternSuggestion {
            specificity: Specificity::Moderate,
            pattern: format!("/{}", moderate_pattern),
            description: "Replace IDs/UUIDs with wildcards".to_string(),
            example_matches: vec![
                path.to_string(),
                generate_example_variant(path, segments),
            ],
        });
    }
    
    // 3. Broad - wildcard all non-static segments
    let static_prefix = find_static_prefix(segments);
    if !static_prefix.is_empty() {
        suggestions.push(PatternSuggestion {
            specificity: Specificity::Broad,
            pattern: format!("/{}/*", static_prefix),
            description: "Match all requests under this path".to_string(),
            example_matches: vec![
                path.to_string(),
                format!("/{}/anything/here", static_prefix),
            ],
        });
    }
    
    suggestions
}
```

## API Changes

### Enhanced `mocks.from_flow`

```json
{
  "method": "mocks.from_flow",
  "params": {
    "flow": 123,
    "pattern_analysis": true,  // NEW: Return pattern suggestions
    "pattern_specificity": "moderate"  // NEW: Auto-apply specificity
  }
}
```

**Response:**
```json
{
  "rule": {...},
  "fixtures": [...],
  "warnings": [],
  "pattern_analysis": {  // NEW
    "original": "/api/v1/users/12345/posts/550e8400-e29b-41d4-a716-446655440000",
    "segments": [
      {"type":"static","value":"api"},
      {"type":"static","value":"v1"},
      {"type":"static","value":"users"},
      {"type":"numeric_id","value":"12345"},
      {"type":"static","value":"posts"},
      {"type":"uuid","value":"550e8400-e29b-41d4-a716-446655440000"}
    ],
    "suggestions": [
      {
        "specificity": "exact",
        "pattern": "/api/v1/users/12345/posts/550e8400-e29b-41d4-a716-446655440000",
        "description": "Match this exact path only"
      },
      {
        "specificity": "moderate",
        "pattern": "/api/v1/users/*/posts/*",
        "description": "Replace IDs/UUIDs with wildcards",
        "example_matches": [
          "/api/v1/users/12345/posts/550e8400-e29b-41d4-a716-446655440000",
          "/api/v1/users/67890/posts/a1b2c3d4-e5f6-7890-abcd-ef1234567890"
        ]
      },
      {
        "specificity": "broad",
        "pattern": "/api/v1/*",
        "description": "Match all v1 API requests"
      }
    ]
  }
}
```

### New Method: `mocks.analyze_path`

```json
{
  "method": "mocks.analyze_path",
  "params": {
    "path": "/api/users/12345/profile"
  }
}
```

**Response:**
```json
{
  "original": "/api/users/12345/profile",
  "segments": [
    {"type":"static","value":"api"},
    {"type":"static","value":"users"},
    {"type":"numeric_id","value":"12345"},
    {"type":"static","value":"profile"}
  ],
  "suggestions": [
    {
      "specificity": "exact",
      "pattern": "/api/users/12345/profile"
    },
    {
      "specificity": "moderate",
      "pattern": "/api/users/*/profile",
      "description": "Match any user's profile"
    },
    {
      "specificity": "broad",
      "pattern": "/api/*",
      "description": "Match all API requests"
    }
  ]
}
```

## Implementation Plan

### 1. Pattern Detection Module

**File:** `crates/netproxyd/src/pattern_detector.rs`

```rust
use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    static ref UUID_REGEX: Regex = Regex::new(
        r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"
    ).unwrap();
    
    static ref DATE_REGEX: Regex = Regex::new(
        r"^\d{4}-\d{2}-\d{2}$"
    ).unwrap();
    
    static ref SLUG_REGEX: Regex = Regex::new(
        r"^[a-z0-9]+(-[a-z0-9]+)+$"
    ).unwrap();
}

const STATIC_KEYWORDS: &[&str] = &[
    "api", "v1", "v2", "v3", "admin", "public", "private",
    "users", "posts", "products", "orders", "items",
    "search", "list", "detail", "profile", "settings",
];

pub struct PatternDetector;

impl PatternDetector {
    pub fn analyze(path: &str) -> PathAnalysis {
        // Implementation
    }
    
    fn classify_segment(segment: &str) -> PathSegmentType {
        // Implementation
    }
    
    fn generate_suggestions(path: &str, segments: &[PathSegmentType]) -> Vec<PatternSuggestion> {
        // Implementation
    }
}
```

### 2. Update Flow Converter

**File:** `crates/netproxyd/src/flow_converter.rs`

```rust
pub struct ConvertOptions {
    // ... existing fields ...
    pub pattern_analysis: bool,           // NEW
    pub pattern_specificity: Option<Specificity>,  // NEW
}

pub struct ConversionResult {
    pub rule: Rule,
    pub fixtures: Vec<Fixture>,
    pub warnings: Vec<String>,
    pub pattern_analysis: Option<PathAnalysis>,  // NEW
}

pub fn convert_flow(
    reader: &Reader,
    run: &str,
    flow_id: u64,
    options: &ConvertOptions,
) -> Result<ConversionResult, ConvertError> {
    // ... existing code ...
    
    // Analyze path if requested
    let pattern_analysis = if options.pattern_analysis {
        Some(PatternDetector::analyze(&flow.path.as_deref().unwrap_or("/")))
    } else {
        None
    };
    
    // Apply pattern specificity if specified
    let final_path = if let Some(specificity) = options.pattern_specificity {
        pattern_analysis.as_ref()
            .and_then(|pa| pa.pattern_suggestions.iter()
                .find(|s| s.specificity == specificity)
                .map(|s| s.pattern.clone()))
            .unwrap_or_else(|| flow.path.clone().unwrap_or_default())
    } else {
        flow.path.clone().unwrap_or_default()
    };
    
    // Use final_path in match_spec
    let match_spec = MatchSpec {
        path: Some(final_path),
        // ... rest of match spec ...
    };
    
    Ok(ConversionResult {
        rule,
        fixtures,
        warnings,
        pattern_analysis,
    })
}
```

### 3. Control API Integration

**File:** `crates/netproxyd/src/control.rs`

```rust
// Add new method
"mocks.analyze_path" => {
    #[derive(Deserialize)]
    struct Params {
        path: String,
    }
    let p: Params = serde_json::from_value(params)?;
    let analysis = PatternDetector::analyze(&p.path);
    Ok(serde_json::to_value(analysis)?)
}

// Update mocks.from_flow to handle pattern_analysis
"mocks.from_flow" => {
    #[derive(Deserialize)]
    struct Params {
        // ... existing fields ...
        pattern_analysis: Option<bool>,
        pattern_specificity: Option<Specificity>,
    }
    // ... use in ConvertOptions ...
}
```

## Testing

### Unit Tests

```rust
#[test]
fn detects_numeric_id() {
    let analysis = PatternDetector::analyze("/api/users/12345");
    assert_eq!(analysis.segments.len(), 3);
    assert!(matches!(analysis.segments[2], PathSegmentType::NumericId));
    
    let moderate = analysis.pattern_suggestions.iter()
        .find(|s| s.specificity == Specificity::Moderate)
        .unwrap();
    assert_eq!(moderate.pattern, "/api/users/*");
}

#[test]
fn detects_uuid() {
    let analysis = PatternDetector::analyze("/orders/550e8400-e29b-41d4-a716-446655440000");
    assert!(matches!(analysis.segments[1], PathSegmentType::Uuid));
}

#[test]
fn detects_date() {
    let analysis = PatternDetector::analyze("/logs/2024-10-17/errors");
    assert!(matches!(analysis.segments[1], PathSegmentType::Date));
}

#[test]
fn generates_three_specificity_levels() {
    let analysis = PatternDetector::analyze("/api/v1/users/123/posts/456");
    assert_eq!(analysis.pattern_suggestions.len(), 3);
    assert!(analysis.pattern_suggestions.iter().any(|s| s.specificity == Specificity::Exact));
    assert!(analysis.pattern_suggestions.iter().any(|s| s.specificity == Specificity::Moderate));
    assert!(analysis.pattern_suggestions.iter().any(|s| s.specificity == Specificity::Broad));
}
```

### Integration Test

```rust
#[tokio::test]
async fn convert_with_pattern_analysis() {
    let daemon = setup_test_daemon().await;
    
    // Capture flow with ID in path
    let flow_id = capture_flow(&daemon, "GET", "https://api.example.com/users/12345").await;
    
    // Convert with pattern analysis
    let result = daemon.convert_flow(flow_id, ConvertOptions {
        pattern_analysis: true,
        pattern_specificity: Some(Specificity::Moderate),
        ..Default::default()
    }).await.unwrap();
    
    // Verify pattern was applied
    assert_eq!(result.rule.match_spec.path, Some("/users/*".to_string()));
    
    // Verify analysis was returned
    let analysis = result.pattern_analysis.unwrap();
    assert_eq!(analysis.suggestions.len(), 3);
}
```

## UI Integration (Swift)

### Pattern Selector UI

```swift
struct PatternSelectorView: View {
    let analysis: PathAnalysis
    @Binding var selectedPattern: String
    
    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Pattern Suggestions")
                .font(.headline)
            
            ForEach(analysis.suggestions, id: \.pattern) { suggestion in
                PatternOptionView(
                    suggestion: suggestion,
                    isSelected: selectedPattern == suggestion.pattern,
                    onSelect: { selectedPattern = suggestion.pattern }
                )
            }
        }
    }
}

struct PatternOptionView: View {
    let suggestion: PatternSuggestion
    let isSelected: Bool
    let onSelect: () -> Void
    
    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    specificityBadge
                    Text(suggestion.pattern)
                        .font(.system(.body, design: .monospaced))
                }
                Text(suggestion.description)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            Spacer()
            if isSelected {
                Image(systemName: "checkmark.circle.fill")
                    .foregroundColor(.blue)
            }
        }
        .padding()
        .background(isSelected ? Color.blue.opacity(0.1) : Color.clear)
        .cornerRadius(8)
        .onTapGesture(perform: onSelect)
    }
    
    var specificityBadge: some View {
        Text(suggestion.specificity.rawValue.capitalized)
            .font(.caption2)
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(badgeColor)
            .foregroundColor(.white)
            .cornerRadius(4)
    }
    
    var badgeColor: Color {
        switch suggestion.specificity {
        case .exact: return .orange
        case .moderate: return .blue
        case .broad: return .purple
        }
    }
}
```

## Examples

### Example 1: User Profile

**Original:** `/api/v1/users/12345/profile`

**Suggestions:**
1. **Exact:** `/api/v1/users/12345/profile` - Only this user
2. **Moderate:** `/api/v1/users/*/profile` - Any user's profile
3. **Broad:** `/api/v1/*` - All v1 API requests

### Example 2: Order Details

**Original:** `/orders/550e8400-e29b-41d4-a716-446655440000/items`

**Suggestions:**
1. **Exact:** `/orders/550e8400-e29b-41d4-a716-446655440000/items`
2. **Moderate:** `/orders/*/items` - Any order's items
3. **Broad:** `/orders/*` - All order endpoints

### Example 3: Blog Post

**Original:** `/blog/2024/10/my-awesome-post`

**Suggestions:**
1. **Exact:** `/blog/2024/10/my-awesome-post`
2. **Moderate:** `/blog/*/*` - Any dated post
3. **Broad:** `/blog/*` - All blog content

## Success Criteria

- ✅ Detect numeric IDs, UUIDs, dates, timestamps, hex, slugs
- ✅ Generate 3 specificity levels for each path
- ✅ API returns pattern analysis
- ✅ Pattern specificity can be auto-applied
- ✅ Unit tests cover all pattern types
- ✅ Integration test validates end-to-end flow

## Timeline Estimate

- **Pattern detection:** 0.5 days
- **Integration with converter:** 0.5 days
- **Control API:** 0.25 days
- **Testing:** 0.5 days
- **Documentation:** 0.25 days

**Total:** ~2 days

---

**Next:** M4 Phase 4 - MCP Integration
