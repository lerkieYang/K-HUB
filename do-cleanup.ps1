Write-Host "=== Before cleanup ==="
$before = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/data/stats"
Write-Host ("knowledge: " + $before.memory.knowledge)
Write-Host ("DB size: " + $before.db_size_mb + " MB")

Write-Host ""
Write-Host "=== Cleaning up old versions ==="
$cleanup = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/data/cleanup-old-versions" -Method Post
Write-Host ($cleanup | ConvertTo-Json)

Write-Host ""
Write-Host "=== After cleanup ==="
$after = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/data/stats"
Write-Host ("knowledge: " + $after.memory.knowledge)
Write-Host ("DB size: " + $after.db_size_mb + " MB")
