Write-Host "=== Unconfiguring all agents ==="

Write-Host "Unconfiguring: hermes"
$r1 = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/agents/unconfigure" -Method POST -Body '{"agent_id":"hermes"}' -ContentType "application/json" -UseBasicParsing).Content
Write-Host $r1

Write-Host "Unconfiguring: codex"
$r2 = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/agents/unconfigure" -Method POST -Body '{"agent_id":"codex"}' -ContentType "application/json" -UseBasicParsing).Content
Write-Host $r2

Write-Host "Unconfiguring: gemini"
$r3 = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/agents/unconfigure" -Method POST -Body '{"agent_id":"gemini"}' -ContentType "application/json" -UseBasicParsing).Content
Write-Host $r3

Write-Host "Unconfiguring: openclaw"
$r4 = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/agents/unconfigure" -Method POST -Body '{"agent_id":"openclaw"}' -ContentType "application/json" -UseBasicParsing).Content
Write-Host $r4

Write-Host ""
Write-Host "=== Verify agent configs ==="
$scan = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/agents/scan" -Method POST -UseBasicParsing).Content | ConvertFrom-Json
foreach ($agent in $scan.agents) {
    Write-Host ($agent.name + " [" + $agent.environment + "] configured=" + $agent.configured)
}

Write-Host ""
Write-Host "=== Check mcp.yaml files ==="
$paths = @(
    "$env:USERPROFILE\.hermes\mcp.yaml",
    "$env:USERPROFILE\.codex\mcp.yaml",
    "$env:USERPROFILE\.gemini\mcp.yaml",
    "$env:USERPROFILE\.openclaw\mcp.yaml"
)
foreach ($p in $paths) {
    if (Test-Path $p) {
        Write-Host ("EXISTS: " + $p)
        Get-Content $p | Write-Host
    } else {
        Write-Host ("GONE:   " + $p)
    }
}
