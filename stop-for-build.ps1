Write-Host "=== Stop current task ==="
$tasksResult = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/knowledge/tasks/stop-all" -Method POST -UseBasicParsing).Content
Write-Host $tasksResult

Start-Sleep -Seconds 2

Write-Host ""
Write-Host "=== Force stop any remaining ==="
$tasksList = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/knowledge/tasks" -UseBasicParsing).Content
Write-Host $tasksList

Write-Host ""
Write-Host "=== Kill hub-service ==="
Stop-Process -Name "hub-service" -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2
Write-Host "Killed"
