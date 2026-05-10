Write-Host "=== Check sample records ==="
$resp = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/memory/search?source_type=knowledge&limit=5"
foreach ($item in $resp.results) {
    $clen = if ($item.content) { $item.content.Length } else { 0 }
    $spath = if ($item.source_file_path) { $item.source_file_path } else { "(null)" }
    Write-Host ("Title: " + $item.title)
    Write-Host ("  content_len: " + $clen)
    Write-Host ("  source_file_path: " + $spath)
    Write-Host ("  tags: " + ($item.tags -join ","))
    Write-Host ("  memory_type: " + $item.memory_type)
    Write-Host ""
}

Write-Host "=== Check a single record by ID ==="
$first = $resp.results[0]
if ($first) {
    $detail = Invoke-RestMethod -Uri ("http://127.0.0.1:8443/api/memory/" + $first.id)
    $dlen = if ($detail.content) { $detail.content.Length } else { 0 }
    Write-Host ("ID: " + $detail.id)
    Write-Host ("  content_len: " + $dlen)
    Write-Host ("  content_preview: " + $detail.content.Substring(0, [Math]::Min(200, $dlen)))
}
