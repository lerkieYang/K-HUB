Write-Host "=== Test new service ==="
$stats = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/data/stats"
Write-Host ("memory.total: " + $stats.memory.total)
Write-Host ("DB size: " + $stats.db_size_mb + " MB")

Write-Host ""
Write-Host "=== Test new endpoints ==="
try {
    $sessionPull = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/session/pull" -Method Post
    Write-Host ("session/pull: " + $sessionPull.total_pulled + " sessions")
} catch {
    Write-Host ("session/pull error: " + $_.Exception.Message)
}

try {
    $memoryPull = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/memory/pull" -Method Post
    Write-Host ("memory/pull: " + $memoryPull.total_pulled + " memories")
} catch {
    Write-Host ("memory/pull error: " + $_.Exception.Message)
}
