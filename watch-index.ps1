param([int]$MaxWait = 120)

$elapsed = 0
while ($elapsed -lt $MaxWait) {
    $tasksResult = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/knowledge/tasks" -UseBasicParsing).Content
    $data = $tasksResult | ConvertFrom-Json
    
    if ($data.total -eq 0) {
        Write-Host "All tasks completed!"
        break
    }
    
    $task = $data.tasks[0]
    $pct = if ($task.total_files -gt 0) { [math]::Round($task.indexed_count / $task.total_files * 100, 1) } else { 0 }
    Write-Host ("[$elapsed s] Stage: $($task.stage) | Indexed: $($task.indexed_count)/$($task.total_files) ($pct%)")
    
    Start-Sleep -Seconds 5
    $elapsed += 5
}

Write-Host ""
Write-Host "=== Final Stats ==="
$statsResult = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/knowledge/stats" -UseBasicParsing).Content
Write-Host $statsResult

Write-Host ""
Write-Host "=== Cache .md files ==="
$cachePath = "C:\Users\lerki\OneDrive\Desktop\knowledgeHUB\apps\hub-service\data\cache\doc_convert"
$mdFiles = Get-ChildItem -Path $cachePath -Filter "*.md" -Recurse -ErrorAction SilentlyContinue
Write-Host ("Total .md files in cache: " + $mdFiles.Count)
