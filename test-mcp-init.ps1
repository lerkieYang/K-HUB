Write-Host "=== Test MCP initialize ==="
$body = @{
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
} | ConvertTo-Json -Depth 10

try {
    $result = Invoke-RestMethod -Uri "http://127.0.0.1:8443/mcp" -Method Post -Body $body -ContentType "application/json"
    Write-Host "MCP initialize works!"
    Write-Host ($result | ConvertTo-Json -Depth 3)
} catch {
    Write-Host ("Error: " + $_.Exception.Message)
}

Write-Host ""
Write-Host "=== Check agent activity after initialize ==="
$activity = Invoke-RestMethod -Uri "http://127.0.0.1:8443/mcp/agent-activity"
Write-Host ($activity | ConvertTo-Json -Depth 5)
