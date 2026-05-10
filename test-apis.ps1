Write-Host "=== API Test Suite ==="
$baseUrl = "http://127.0.0.1:8443"
$passed = 0
$failed = 0

function Test-Endpoint {
    param([string]$Name, [string]$Method, [string]$Url, [string]$Body)
    try {
        $params = @{
            Uri = $Url
            Method = $Method
            UseBasicParsing = $true
            TimeoutSec = 10
        }
        if ($Body) {
            $params.Body = $Body
            $params.ContentType = "application/json"
        }
        $response = Invoke-WebRequest @params
        Write-Host "  PASS: $Name (HTTP $($response.StatusCode))"
        $script:passed++
        return $response.Content
    } catch {
        Write-Host "  FAIL: $Name - $($_.Exception.Message)"
        $script:failed++
        return $null
    }
}

Write-Host ""
Write-Host "--- Health & Stats ---"
Test-Endpoint "Health" "GET" "$baseUrl/api/knowledge/stats"
Test-Endpoint "Data Stats" "GET" "$baseUrl/api/data/stats"
Test-Endpoint "Progress" "GET" "$baseUrl/api/knowledge/progress"

Write-Host ""
Write-Host "--- Knowledge Config ---"
Test-Endpoint "List Configs" "GET" "$baseUrl/api/knowledge/config"

Write-Host ""
Write-Host "--- Memory Search ---"
$searchResult = Test-Endpoint "Search Knowledge" "GET" "$baseUrl/api/memory/search?source_type=knowledge&limit=5"
if ($searchResult) {
    $data = $searchResult | ConvertFrom-Json
    Write-Host "    Results: $($data.results.Count)"
}

Write-Host ""
Write-Host "--- Tasks ---"
Test-Endpoint "List Tasks" "GET" "$baseUrl/api/knowledge/tasks"

Write-Host ""
Write-Host "--- MCP ---"
Test-Endpoint "MCP Tools" "GET" "$baseUrl/mcp/tools"
Test-Endpoint "Agent Activity" "GET" "$baseUrl/mcp/agent-activity"

Write-Host ""
Write-Host "--- Agent Scan ---"
Test-Endpoint "Agent Scan" "POST" "$baseUrl/api/agents/scan"

Write-Host ""
Write-Host "--- Cleanup ---"
Test-Endpoint "Cleanup Old Versions" "POST" "$baseUrl/api/data/cleanup-old-versions"

Write-Host ""
Write-Host "=== Results: $passed passed, $failed failed ==="
