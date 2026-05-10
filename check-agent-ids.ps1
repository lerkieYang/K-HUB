Write-Host "=== Agent scan results ==="
$scan = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/agents/scan" -Method Post
foreach ($a in $scan.agents) {
    Write-Host ("ID: " + $a.id + " | Name: " + $a.name + " | Configured: " + $a.configured)
}

Write-Host ""
Write-Host "=== Agent activity ==="
$activity = Invoke-RestMethod -Uri "http://127.0.0.1:8443/mcp/agent-activity"
foreach ($item in $activity.agents) {
    Write-Host ("Agent: " + $item.agent_id + " | Last seen: " + $item.last_seen)
}
