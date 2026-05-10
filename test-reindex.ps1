Write-Host "=== Triggering reindex ==="
$reindexResult = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/knowledge/reindex" -Method POST -UseBasicParsing).Content
Write-Host $reindexResult

Start-Sleep -Seconds 3

Write-Host ""
Write-Host "=== Tasks after reindex ==="
$tasksResult = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/knowledge/tasks" -UseBasicParsing).Content
Write-Host $tasksResult

Write-Host ""
Write-Host "=== Progress ==="
$progressResult = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/knowledge/progress" -UseBasicParsing).Content
Write-Host $progressResult

Start-Sleep -Seconds 10

Write-Host ""
Write-Host "=== Progress after 10s ==="
$progressResult2 = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/knowledge/progress" -UseBasicParsing).Content
Write-Host $progressResult2
