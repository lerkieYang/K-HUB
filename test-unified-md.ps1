Write-Host "=== Health ==="
try {
    $health = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/knowledge/stats" -UseBasicParsing).Content
    Write-Host $health
} catch {
    Write-Host "Service not ready"
}

Write-Host ""
Write-Host "=== Check cache directory ==="
$cachePath = "C:\Users\lerki\OneDrive\Desktop\knowledgeHUB\apps\hub-service\data\cache\doc_convert"
if (Test-Path $cachePath) {
    $mdFiles = Get-ChildItem -Path $cachePath -Filter "*.md" -Recurse -ErrorAction SilentlyContinue
    Write-Host ("Total .md files in cache: " + $mdFiles.Count)
    Write-Host ""
    Write-Host "Sample files (first 10):"
    $mdFiles | Select-Object -First 10 | ForEach-Object {
        Write-Host ("  " + $_.FullName + " (" + [math]::Round($_.Length/1024, 1) + " KB)")
    }
} else {
    Write-Host "Cache directory not found"
}

Write-Host ""
Write-Host "=== Trigger reindex ==="
$reindexResult = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/knowledge/reindex" -Method POST -UseBasicParsing).Content
Write-Host $reindexResult
