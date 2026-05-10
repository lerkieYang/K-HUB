Write-Host "=== 1. Check agent activity endpoint ==="
$activity = Invoke-RestMethod -Uri "http://127.0.0.1:8443/mcp/agent-activity"
Write-Host ($activity | ConvertTo-Json -Depth 5)

Write-Host ""
Write-Host "=== 2. Check MCP tools endpoint ==="
$tools = Invoke-RestMethod -Uri "http://127.0.0.1:8443/mcp/tools"
Write-Host ("Tools count: " + $tools.Count)
foreach ($t in $tools) {
    Write-Host ("  - " + $t.name)
}

Write-Host ""
Write-Host "=== 3. Check agents scan ==="
$agents = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/agents/scan"
Write-Host ("Agents found: " + $agents.agents.Count)
foreach ($a in $agents.agents) {
    Write-Host ("  - " + $a.agent_id + " | " + $a.name + " | configured: " + $a.configured)
}

Write-Host ""
Write-Host "=== 4. Test MCP call (simulate agent) ==="
$body = @{
    jsonrpc = "2.0"
    method = "tools/call"
    id = 1
    params = @{
        name = "knowledge.search"
        arguments = @{
            query = "test"
            limit = 1
        }
    }
} | ConvertTo-Json -Depth 5
$headers = @{ "Content-Type" = "application/json" }
try {
    $mcpResult = Invoke-RestMethod -Uri "http://127.0.0.1:8443/mcp" -Method Post -Body $body -Headers $headers
    Write-Host "MCP call result:"
    Write-Host ($mcpResult | ConvertTo-Json -Depth 3)
} catch {
    Write-Host ("MCP call error: " + $_.Exception.Message)
}

Write-Host ""
Write-Host "=== 5. Check agent activity again ==="
$activity2 = Invoke-RestMethod -Uri "http://127.0.0.1:8443/mcp/agent-activity"
Write-Host ($activity2 | ConvertTo-Json -Depth 5)
