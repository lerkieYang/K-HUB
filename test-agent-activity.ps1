Write-Host "=== 1. Initialize (simulate Gemini) ==="
$initBody = @{
    jsonrpc = "2.0"
    method = "initialize"
    id = 1
    params = @{
        protocolVersion = "2024-11-05"
        capabilities = @{}
        clientInfo = @{
            name = "gemini-cli"
            version = "1.0.0"
        }
    }
} | ConvertTo-Json -Depth 5
$headers = @{ "Content-Type" = "application/json" }
$initResult = Invoke-RestMethod -Uri "http://127.0.0.1:8443/mcp" -Method Post -Body $initBody -Headers $headers
Write-Host "Initialize result:"
Write-Host ($initResult | ConvertTo-Json -Depth 3)

Write-Host ""
Write-Host "=== 2. Call knowledge.search (simulate Gemini) ==="
$searchBody = @{
    jsonrpc = "2.0"
    method = "tools/call"
    id = 2
    params = @{
        name = "knowledge.search"
        arguments = @{
            query = "test"
            limit = 2
            agent_id = "gemini-cli"
        }
    }
} | ConvertTo-Json -Depth 5
$searchResult = Invoke-RestMethod -Uri "http://127.0.0.1:8443/mcp" -Method Post -Body $searchBody -Headers $headers
Write-Host "Search result (truncated):"
$text = $searchResult.result.content[0].text
Write-Host $text.Substring(0, [Math]::Min(300, $text.Length))

Write-Host ""
Write-Host "=== 3. Check agent activity ==="
$activity = Invoke-RestMethod -Uri "http://127.0.0.1:8443/mcp/agent-activity"
Write-Host ($activity | ConvertTo-Json -Depth 5)

Write-Host ""
Write-Host "=== 4. Check agents page data ==="
$agents = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/agents/scan"
Write-Host ("Agents found: " + $agents.agents.Count)
foreach ($a in $agents.agents) {
    Write-Host ("  - " + $a.agent_id + " | " + $a.name + " | configured: " + $a.configured)
}
