#!/bin/bash
# Comprehensive test suite for scry - tests one log type at a time

# Get test number from argument, or random if not provided
TEST_NUM=${1:-$((RANDOM % 32 + 1))}

# Function to generate logs for a specific test case
generate_test() {
    local test_num=$1
    
    case $test_num in
        1)
            echo "=== TEST TYPE: Simple JSON logs ===" >&2
            echo '{"level":"INFO","msg":"test","count":123}'
            echo '{"level":"ERROR","msg":"failed","code":500}'
            echo '{"level":"DEBUG","msg":"processing","step":2}'
            ;;
        2)
            echo "=== TEST TYPE: Nested JSON objects ===" >&2
            echo '{"request":{"method":"GET","path":"/api","headers":{"user-agent":"curl"}}}'
            echo '{"user":{"id":123,"profile":{"name":"John","age":30}}}'
            echo '{"config":{"database":{"host":"localhost","port":5432}}}'
            ;;
        3)
            echo "=== TEST TYPE: JSON with arrays ===" >&2
            echo '{"tags":["urgent","production"],"ids":[1,2,3,4,5]}'
            echo '{"items":["apple","banana","cherry"],"count":3}'
            echo '{"errors":["error1","error2"],"warnings":[]}'
            ;;
        4)
            echo "=== TEST TYPE: Key-value pairs ===" >&2
            echo "level=INFO msg=request_processed duration=150ms status=200"
            echo "user_id=12345 action=login timestamp=2024-01-01T12:00:00Z"
            echo "count=42 price=99.99 enabled=true"
            ;;
        5)
            echo "=== TEST TYPE: Mixed key-value and JSON ===" >&2
            echo '{"type":"event"} level=DEBUG component=api'
            echo "timestamp=1234567890 data={\"key\":\"value\"}"
            echo '{"json":"data"} key=value extra=info'
            ;;
        6)
            echo "=== TEST TYPE: Plain text logs ===" >&2
            echo "[2024-01-01 12:00:00] Application started"
            echo "[2024-01-01 12:00:01] Processing request #12345"
            echo "[2024-01-01 12:00:02] Request completed successfully"
            ;;
        7)
            echo "=== TEST TYPE: Logs with special characters ===" >&2
            echo '{"message":"Path: /usr/bin/test & check"}'
            echo "error=File not found: /tmp/test'file.txt"
            echo '{"query":"SELECT * FROM users WHERE id=1"}'
            ;;
        8)
            echo "=== TEST TYPE: Unicode and emojis ===" >&2
            echo '{"msg":"测试日志","user":"José","emoji":"🚀"}'
            echo "message=Hello 世界 🌍 status=ok"
            echo '{"text":"Привет мир","symbol":"©®™"}'
            ;;
        9)
            echo "=== TEST TYPE: Very long lines ===" >&2
            echo "{\"data\":\"$(printf 'x%.0s' {1..500})\"}"
            echo "key=value$(printf 'x%.0s' {1..1000})"
            echo "short=normal"
            ;;
        10)
            echo "=== TEST TYPE: Empty and whitespace-only lines ===" >&2
            echo ""
            echo "   "
            echo "normal log after empty"
            echo "another normal log"
            ;;
        11)
            echo "=== TEST TYPE: Numbers in various formats ===" >&2
            echo '{"int":123,"float":45.67,"scientific":1.23e10,"negative":-42}'
            echo "count=1e6 price=99.99 ratio=-0.5"
            echo '{"big":9223372036854775807,"small":0.0000001}'
            ;;
        12)
            echo "=== TEST TYPE: Boolean and null values ===" >&2
            echo '{"enabled":true,"disabled":false,"data":null}'
            echo "active=true deleted=false value=null"
            echo '{"check":true,"unset":null}'
            ;;
        13)
            echo "=== TEST TYPE: Dates and timestamps ===" >&2
            echo '{"date":"2024-01-01","time":"12:00:00","iso":"2024-01-01T12:00:00Z"}'
            echo "created=2024-01-01 updated=1763571293"
            echo '{"timestamp":1763571293000,"epoch":1234567890}'
            ;;
        14)
            echo "=== TEST TYPE: Escaped characters ===" >&2
            echo '{"path":"C:\\Users\\test","quote":"He said \"hello\""}'
            echo "json={\"escaped\":\"value\\nwith\\tchars\"}"
            echo '{"text":"line1\\nline2","tab":"col1\\tcol2"}'
            ;;
        15)
            echo "=== TEST TYPE: Malformed JSON (should still display) ===" >&2
            echo '{"incomplete":'
            echo '{"missing":quote}'
            echo '{invalid json}'
            echo '{"valid":"json"}'
            ;;
        16)
            echo "=== TEST TYPE: Control characters ===" >&2
            printf "normal\twith\ttabs\n"
            printf "line with\nnewline\n"
            echo "normal after control chars"
            ;;
        17)
            echo "=== TEST TYPE: SQL-like logs ===" >&2
            echo "SELECT * FROM users WHERE id=123"
            echo "INSERT INTO logs VALUES ('test', 123, NOW())"
            echo "UPDATE users SET status='active' WHERE id=456"
            ;;
        18)
            echo "=== TEST TYPE: HTTP request logs ===" >&2
            echo '{"method":"POST","url":"/api/users","status":201,"ip":"192.168.1.1"}'
            echo "GET /api/status HTTP/1.1 200 OK"
            echo '{"request":"GET /api","response":200,"duration":45}'
            ;;
        19)
            echo "=== TEST TYPE: Stack traces (multi-line style) ===" >&2
            echo "ERROR: Exception occurred"
            echo "  at com.example.Test.main(Test.java:42)"
            echo "  at java.lang.Thread.run(Thread.java:748)"
            echo "WARNING: Another error"
            ;;
        20)
            echo "=== TEST TYPE: Docker/container logs ===" >&2
            echo "[2024-01-01T12:00:00.123Z] container=web-1 level=info msg=started"
            echo '{"container":"db-1","level":"error","message":"connection failed"}'
            echo "[2024-01-01T12:00:01.456Z] container=app level=debug msg=processing"
            ;;
        21)
            echo "=== TEST TYPE: Apache/Nginx access logs ===" >&2
            echo '127.0.0.1 - - [01/Jan/2024:12:00:00 +0000] "GET / HTTP/1.1" 200 1234'
            echo "192.168.1.1 - user [01/Jan/2024:12:00:01 +0000] \"POST /api\" 201 5678"
            echo '10.0.0.1 - admin [01/Jan/2024:12:00:02 +0000] "PUT /resource" 204 0'
            ;;
        22)
            echo "=== TEST TYPE: Kubernetes logs ===" >&2
            echo '{"level":"info","ts":1234567890,"caller":"main.go:42","msg":"pod started"}'
            echo "k8s pod=web-deployment-123 container=app level=info"
            echo '{"namespace":"default","pod":"web-1","container":"app","level":"debug"}'
            ;;
        23)
            echo "=== TEST TYPE: JSON with deeply nested structures ===" >&2
            echo '{"a":{"b":{"c":{"d":{"e":"deep"}}}}}'
            echo '{"config":{"database":{"host":"localhost","port":5432,"ssl":true}}}'
            echo '{"level1":{"level2":{"level3":{"level4":"value"}}}}'
            ;;
        24)
            echo "=== TEST TYPE: Logs with URLs and paths ===" >&2
            echo '{"url":"https://example.com/api?key=value&test=123"}'
            echo "path=/usr/local/bin/app config=/etc/app/config.json"
            echo '{"endpoint":"https://api.example.com/v1/users?page=1&limit=10"}'
            ;;
        25)
            echo "=== TEST TYPE: Base64 encoded data ===" >&2
            echo '{"data":"SGVsbG8gV29ybGQ=","type":"base64"}'
            echo "encoded=dGVzdCBkYXRh status=ok"
            echo '{"payload":"YWJjZGVmZ2g=","format":"base64"}'
            ;;
        26)
            echo "=== TEST TYPE: JSON with all value types mixed ===" >&2
            echo '{"str":"text","num":123,"bool":true,"null":null,"arr":[1,2],"obj":{"key":"val"}}'
            echo '{"mixed":"types","count":42,"active":false,"tags":["a","b"],"meta":{}}'
            ;;
        27)
            echo "=== TEST TYPE: Logs with quotes and special JSON chars ===" >&2
            echo '{"message":"User said: \"Hello, world!\""}'
            echo "text=He said \"hello\" and left"
            echo '{"quote":"She said '\''yes'\'' and smiled"}'
            ;;
        28)
            echo "=== TEST TYPE: Very large numbers ===" >&2
            echo '{"timestamp":1763571293000,"id":9223372036854775807}'
            echo "big_num=999999999999999999 small=0.0000001"
            echo '{"huge":999999999999999999999,"tiny":0.0000000001}'
            ;;
        29)
            echo "=== TEST TYPE: Mixed case and formatting ===" >&2
            echo '{"Level":"INFO","MSG":"test","Count":123}'
            echo "LEVEL=DEBUG MSG=processing COUNT=42"
            echo '{"Case":"Mixed","format":"Varied"}'
            ;;
        30)
            echo "=== TEST TYPE: Edge case - single character values ===" >&2
            echo '{"a":"x","b":1,"c":true}'
            echo "x=y a=1 b=true"
            echo '{"min":"","max":"z","num":0}'
            ;;
        31)
            echo "=== TEST TYPE: Logs with brackets and braces ===" >&2
            echo '{"path":"/var/log/app[1].log","config":"{key:value}"}'
            echo "file=[test].log config={nested:value}"
            echo '{"array":"[1,2,3]","object":"{a:b}"}'
            ;;
        32)
            echo "=== TEST TYPE: Empty JSON objects and arrays ===" >&2
            echo '{}'
            echo '[]'
            echo '{"empty_obj":{},"empty_arr":[]}'
            echo '{"has_data":"value","empty":{}}'
            ;;
        *)
            echo "=== TEST TYPE: Random mix of all types ===" >&2
            echo '{"level":"INFO","msg":"test"}'
            echo "key=value status=ok"
            echo "[2024-01-01] Plain log line"
            ;;
    esac
}

# Generate the test
generate_test $TEST_NUM

# Instructions
echo "" >&2
echo "Test case $TEST_NUM of 32" >&2
echo "Press 'a' in scry to analyze these logs" >&2
echo "Usage: ./test_logs.sh [NUMBER]" >&2
echo "  NUMBER: 1-32 for specific test, or omit for random" >&2
