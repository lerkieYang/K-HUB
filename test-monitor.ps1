param([int]$WaitSeconds = 30)

Write-Host "=== Waiting $WaitSeconds seconds for indexing to progress ==="
Start-Sleep -Seconds $WaitSeconds

Write-Host ""
Write-Host "=== Tasks ==="
$tasksResult = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/knowledge/tasks" -UseBasicParsing).Content
Write-Host $tasksResult

Write-Host ""
Write-Host "=== Progress ==="
$progressResult = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/knowledge/progress" -UseBasicParsing).Content
Write-Host $progressResult
