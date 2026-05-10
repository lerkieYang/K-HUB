Write-Host "=== 1. Knowledge base stats ==="
$stats = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/knowledge/stats" -UseBasicParsing).Content
Write-Host $stats

Write-Host ""
Write-Host "=== 2. Sample records (check content) ==="
$search = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/memory/search?source_type=knowledge&limit=10" -UseBasicParsing).Content
$data = $search | ConvertFrom-Json
Write-Host ("Total results: " + $data.results.Count)
Write-Host ""

foreach ($item in $data.results) {
    $contentLen = if ($item.content) { $item.content.Length } else { 0 }
    $titleLen = if ($item.title) { $item.title.Length } else { 0 }
    Write-Host ("Title: " + $item.title.Substring(0, [Math]::Min(60, $titleLen)) + "...")
    Write-Host ("  Content length: " + $contentLen + " chars")
    Write-Host ("  Tags: " + ($item.tags -join ", "))
    Write-Host ("  Memory type: " + $item.memory_type)
    Write-Host ("  Source file: " + $item.source_file_path)
    Write-Host ""
}

Write-Host ""
Write-Host "=== 3. Check empty content records ==="
$emptySearch = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/memory/search?source_type=knowledge&limit=200" -UseBasicParsing).Content
$emptyData = $emptySearch | ConvertFrom-Json
$emptyCount = ($emptyData.results | Where-Object { -not $_.content -or $_.content.Length -lt 10 }).Count
$shortCount = ($emptyData.results | Where-Object { $_.content -and $_.content.Length -lt 50 }).Count
Write-Host ("Records with empty/very short content (<10 chars): " + $emptyCount)
Write-Host ("Records with short content (<50 chars): " + $shortCount)

Write-Host ""
Write-Host "=== 4. Memory type distribution ==="
$allSearch = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/memory/search?source_type=knowledge&limit=200" -UseBasicParsing).Content
$allData = $allSearch | ConvertFrom-Json
$allData.results | Group-Object memory_type | ForEach-Object {
    Write-Host ("  " + $_.Name + ": " + $_.Count)
}

Write-Host ""
Write-Host "=== 5. Tags distribution ==="
$tagged = ($allData.results | Where-Object { $_.tags -and $_.tags.Count -gt 0 }).Count
$untagged = ($allData.results | Where-Object { -not $_.tags -or $_.tags.Count -eq 0 }).Count
Write-Host ("  With tags: " + $tagged)
Write-Host ("  Without tags: " + $untagged)
