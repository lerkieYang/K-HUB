Write-Host "=== 1. Knowledge Stats ==="
$stats = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/knowledge/stats"
Write-Host ("total_memories: " + $stats.total_memories)
Write-Host ("embedded_count: " + $stats.embedded_count)
Write-Host ("pending_embedding: " + $stats.pending_embedding)

Write-Host ""
Write-Host "=== 2. Progress ==="
$progress = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/knowledge/progress"
Write-Host ("total_files: " + $progress.total_files)
Write-Host ("indexed_count: " + $progress.indexed_count)
Write-Host ("configs_count: " + $progress.configs_count)

Write-Host ""
Write-Host "=== 3. Memory stats (all source_types) ==="
$memStats = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/memory/stats"
Write-Host ($memStats | ConvertTo-Json -Depth 5)

Write-Host ""
Write-Host "=== 4. Search with high limit ==="
$search = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/memory/search?source_type=knowledge&limit=500"
Write-Host ("search results count: " + $search.results.Count)
Write-Host ("search total: " + $search.total)

Write-Host ""
Write-Host "=== 5. Check is_current distribution ==="
Write-Host "(Need to check DB directly)"

Write-Host ""
Write-Host "=== 6. Knowledge configs ==="
$configs = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/knowledge/config"
foreach ($c in $configs.configs) {
    Write-Host ("  " + $c.name + " | paths: " + ($c.paths -join ", "))
}
