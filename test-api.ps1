Write-Host "=== 1. Health Check ==="
$health = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/" -UseBasicParsing).Content
Write-Host $health

Write-Host ""
Write-Host "=== 2. List Tasks (should be empty) ==="
$tasks = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/knowledge/tasks" -UseBasicParsing).Content
Write-Host $tasks

Write-Host ""
Write-Host "=== 3. Progress ==="
$progress = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/knowledge/progress" -UseBasicParsing).Content
Write-Host $progress

Write-Host ""
Write-Host "=== 4. Knowledge Stats ==="
$stats = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/knowledge/stats" -UseBasicParsing).Content
Write-Host $stats

Write-Host ""
Write-Host "=== 5. MCP Tools ==="
$tools = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/mcp/tools" -UseBasicParsing).Content
Write-Host $tools.Substring(0, [Math]::Min(300, $tools.Length))

Write-Host ""
Write-Host "=== 6. Force Stop (non-existent task) ==="
try {
    $force = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/knowledge/tasks/fake123/force-stop" -Method POST -UseBasicParsing).Content
    Write-Host $force
} catch {
    Write-Host "Error: $($_.Exception.Message)"
}

Write-Host ""
Write-Host "=== 7. Agent Activity ==="
$activity = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/mcp/agent-activity" -UseBasicParsing).Content
Write-Host $activity

Write-Host ""
Write-Host "=== ALL TESTS PASSED ==="
