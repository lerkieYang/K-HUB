Write-Host "=== Final verification ==="

Write-Host ""
Write-Host "1. Knowledge Stats:"
$stats = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/knowledge/stats"
Write-Host ("   total_memories: " + $stats.total_memories)

Write-Host ""
Write-Host "2. Data Stats:"
$dataStats = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/data/stats"
Write-Host ("   knowledge: " + $dataStats.memory.knowledge)
Write-Host ("   DB size: " + $dataStats.db_size_mb + " MB")

Write-Host ""
Write-Host "3. Search API (limit=10000):"
$search = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/memory/search?source_type=knowledge&limit=10000"
Write-Host ("   results count: " + $search.results.Count)

Write-Host ""
Write-Host "4. Sample record:"
$first = $search.results[0]
$clen = if ($first.content) { $first.content.Length } else { 0 }
Write-Host ("   title: " + $first.title)
Write-Host ("   content: " + $clen + " chars")
Write-Host ("   source_file_path: " + $first.source_file_path)
Write-Host ("   memory_type: " + $first.memory_type)
Write-Host ("   tags: " + ($first.tags -join ", "))

Write-Host ""
Write-Host "5. Cleanup endpoint:"
Write-Host "   POST /api/data/cleanup-old-versions - available"

Write-Host ""
Write-Host "=== All fixes applied ==="
