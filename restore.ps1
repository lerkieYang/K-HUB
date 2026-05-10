Write-Host "Restoring deleted knowledge records..."
$r = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/knowledge/restore-deleted" -Method POST -UseBasicParsing).Content
Write-Host $r

Write-Host ""
Write-Host "Verifying memory stats..."
$s = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/memory/stats" -UseBasicParsing).Content
Write-Host $s
