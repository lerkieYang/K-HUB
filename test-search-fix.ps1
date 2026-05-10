Write-Host "=== Test search API ==="
$resp = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/memory/search?source_type=knowledge&limit=5"
Write-Host ("Total results: " + $resp.results.Count)
Write-Host ""

foreach ($item in $resp.results) {
    $clen = if ($item.content) { $item.content.Length } else { 0 }
    $cplen = if ($item.content_preview) { $item.content_preview.Length } else { 0 }
    $spath = if ($item.source_file_path) { $item.source_file_path } else { "(null)" }
    Write-Host ("Title: " + $item.title)
    Write-Host ("  content: " + $clen + " chars")
    Write-Host ("  content_preview: " + $cplen + " chars")
    Write-Host ("  source_file_path: " + $spath)
    Write-Host ("  memory_type: " + $item.memory_type)
    Write-Host ("  tags: " + ($item.tags -join ", "))
    Write-Host ""
}

Write-Host "=== Test detail API ==="
$first = $resp.results[0]
if ($first) {
    $detail = Invoke-RestMethod -Uri ("http://127.0.0.1:8443/api/memory/" + $first.id)
    $dlen = if ($detail.content) { $detail.content.Length } else { 0 }
    Write-Host ("ID: " + $detail.id)
    Write-Host ("  content length: " + $dlen)
    Write-Host ("  content preview: " + $detail.content.Substring(0, [Math]::Min(200, $dlen)))
}
