Write-Host "=== Before clear ==="
$before = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/data/stats"
Write-Host ("memory.total: " + $before.memory.total)
Write-Host ("memory.knowledge: " + $before.memory.knowledge)

Write-Host ""
Write-Host "=== Clearing memory ==="
$clear = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/data/clear" -Method Post -Body '{"clear_memory": true}' -ContentType "application/json"
Write-Host ($clear | ConvertTo-Json)

Write-Host ""
Write-Host "=== After clear ==="
$after = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/data/stats"
Write-Host ("memory.total: " + $after.memory.total)
Write-Host ("memory.knowledge: " + $after.memory.knowledge)

Write-Host ""
Write-Host "=== Refresh again ==="
$refresh = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/data/stats"
Write-Host ("memory.total: " + $refresh.memory.total)
