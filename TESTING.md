# Testing scry - Comprehensive Test Suite

This document describes how to test scry with various log formats and edge cases.

## Quick Test

Run a specific test case (1-32):

```bash
./test_logs.sh 1 | cargo run    # Test case 1: Simple JSON logs
./test_logs.sh 5 | cargo run    # Test case 5: Mixed formats
./test_logs.sh 8 | cargo run    # Test case 8: Unicode and emojis
```

Or run a random test:

```bash
./test_logs.sh | cargo run       # Random test case
```

The script will output which test type is being run, then you can press `a` in scry to analyze.

## Test Cases Included

The test suite includes 32 different test cases covering:

1. **Simple JSON logs** - Basic JSON structure
2. **Nested JSON objects** - Deep nesting
3. **JSON with arrays** - Array handling
4. **Key-value pairs** - Standard key=value format
5. **Mixed formats** - JSON and key-value in same line
6. **Plain text logs** - Unstructured text
7. **Special characters** - Quotes, ampersands, etc.
8. **Unicode and emojis** - International characters
9. **Very long lines** - Truncation handling
10. **Empty lines** - Whitespace handling
11. **Numbers** - Various number formats
12. **Booleans and null** - Special JSON values
13. **Dates and timestamps** - Time formats
14. **Escaped characters** - Backslashes, quotes
15. **Malformed JSON** - Invalid JSON that should still display
16. **Control characters** - Tabs, newlines
17. **SQL-like logs** - Database query logs
18. **HTTP request logs** - Web server logs
19. **Stack traces** - Multi-line error logs
20. **Docker/container logs** - Containerized app logs
21. **Apache/Nginx logs** - Web server access logs
22. **Kubernetes logs** - K8s pod logs
23. **Deeply nested JSON** - Multiple levels of nesting
24. **URLs and paths** - File paths and URLs
25. **Base64 data** - Encoded data
26. **Mixed value types** - All JSON types together
27. **Quotes in strings** - Escaped quotes
28. **Very large numbers** - Big integers and small floats
29. **Mixed case** - Different capitalization
30. **Single character values** - Minimal data
31. **Brackets and braces** - Special characters in values
32. **Empty objects/arrays** - Edge case JSON structures

## Manual Testing

You can also test individual cases:

```bash
# Test JSON
echo '{"level":"INFO","msg":"test"}' | cargo run

# Test key-value
echo "level=INFO msg=test" | cargo run

# Test with special chars
echo '{"msg":"test & check"}' | cargo run

# Test unicode
echo '{"msg":"测试"}' | cargo run
```

## Expected Behavior

scry should:
- ✅ Display ALL data (no hiding)
- ✅ Handle any character set (unicode, emojis, control chars)
- ✅ Show values in JSON view (not just keys)
- ✅ Gracefully handle malformed JSON
- ✅ Not crash on any input
- ✅ Sanitize control characters for display
- ✅ Truncate only extremely long values (with indication)
- ✅ Switch views correctly based on log format

## Reporting Issues

If scry fails on any test case:
1. Note which test case failed
2. Check if it crashes or just displays incorrectly
3. Report the issue with the specific log line that caused problems
