Write-Host "=== Clearing all data (keeping AI config) ==="
$clearResult = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/data/clear" -Method POST -Body '{"clear_all": true}' -ContentType "application/json" -UseBasicParsing).Content
Write-Host $clearResult

Write-Host ""
Write-Host "=== Verify AI config preserved ==="
$aiResult = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/ai/config" -UseBasicParsing).Content
Write-Host $aiResult

Write-Host ""
Write-Host "=== Verify data cleared ==="
$statsResult = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/data/stats" -UseBasicParsing).Content
Write-Host $statsResult

Write-Host ""
Write-Host "=== Verify prompt updated ==="
$promptResult = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/agents/hermes/config-prompt" -UseBasicParsing).Content
Write-Host ($promptResult | ConvertFrom-Json | Select-Object -ExpandProperty prompt | Select-Object -First 5)
