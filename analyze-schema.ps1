Write-Host "=== 1. Current DB schema ==="
$dbPath = "C:\Users\lerki\OneDrive\Desktop\knowledgeHUB\apps\hub-service\data\knowledge-hub.db"

Write-Host ""
Write-Host "=== 2. Memory table source_type distribution ==="
$stats = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/memory/stats"
Write-Host ($stats.by_source | ConvertTo-Json)

Write-Host ""
Write-Host "=== 3. Knowledge configs ==="
$configs = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/knowledge/config"
foreach ($c in $configs.configs) {
    Write-Host ("  ID: " + $c.id)
    Write-Host ("  Name: " + $c.name)
    Write-Host ("  Paths: " + ($c.paths -join ", "))
    Write-Host ""
}

Write-Host ""
Write-Host "=== 4. Knowledge stats ==="
$kbStats = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/knowledge/stats"
Write-Host ("total_memories: " + $kbStats.total_memories)

Write-Host ""
Write-Host "=== 5. Data stats ==="
$dataStats = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/data/stats"
Write-Host ("memory.total: " + $dataStats.memory.total)
Write-Host ("memory.active: " + $dataStats.memory.active)
Write-Host ("memory.knowledge: " + $dataStats.memory.knowledge)
Write-Host ("config_count: " + $dataStats.config_count)
