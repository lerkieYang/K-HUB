Write-Host "=== 1. Check agent activity ==="
$activity = Invoke-RestMethod -Uri "http://127.0.0.1:8443/mcp/agent-activity"
Write-Host ($activity | ConvertTo-Json -Depth 5)

Write-Host ""
Write-Host "=== 2. Check MCP tools ==="
$tools = Invoke-RestMethod -Uri "http://127.0.0.1:8443/mcp/tools"
Write-Host ("Tools count: " + $tools.Count)
foreach ($t in $tools) {
    Write-Host ("  - " + $t.name)
}

Write-Host ""
Write-Host "=== 3. Test MCP endpoint directly ==="
$body = @{
    jsonrpc = "2.0"
    method = "tools/list"
    id = 1
} | ConvertTo-Json

try {
    $result = Invoke-RestMethod -Uri "http://127.0.0.1:8443/mcp" -Method Post -Body $body -ContentType "application/json"
    Write-Host "MCP endpoint works!"
    Write-Host ($result | ConvertTo-Json -Depth 3)
} catch {
    Write-Host ("MCP endpoint error: " + $_.Exception.Message)
}

Write-Host ""
Write-Host "=== 4. Check if MCP needs auth ==="
Write-Host "Checking MCP_TOKEN env..."
$mcpToken = [System.Environment]::GetEnvironmentVariable("MCP_TOKEN")
if ($mcpToken) {
    Write-Host "MCP_TOKEN is set"
} else {
    Write-Host "MCP_TOKEN is NOT set (loopback only)"
}
