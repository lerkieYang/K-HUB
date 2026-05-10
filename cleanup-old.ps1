Write-Host "=== Clean up old versions ==="
Write-Host "Before:"
$before = Invoke-RestMethod -Uri "http://127.0.0.1:8443/api/data/stats"
Write-Host ("  knowledge total: " + $before.memory.knowledge)
Write-Host ("  DB size: " + $before.db_size_mb + " MB")

# Delete old versions (is_current = 0) via direct SQL
# We need to use the clear API or add a new endpoint
# For now, let's check how many old versions exist
Write-Host ""
Write-Host "=== Checking old version count ==="
# The memory stats shows 14685 total knowledge, but stats shows 6266 current
# So 14685 - 6266 = 8419 old versions
Write-Host ("Estimated old versions: " + (14685 - 6266))
