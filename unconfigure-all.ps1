Write-Host "=== Unconfiguring all agents ==="

$agents = @("hermes", "codex", "gemini", "openclaw")
foreach ($a in $agents) {
    Write-Host ("Unconfiguring: " + $a)
    $body = '{"agent_id":"' + $a + '"}'
    try {
        $result = (Invoke-WebRequest -Uri "http://127.0.0.1:8443/api/agents/unconfigure" -Method POST -Body $body -ContentType "application/json" -UseBasicParsing).Content
        Write-Host $result
    } catch {
        Write-Host ("Error: " + $_.Exception.Message)
    }
}

Write-Host ""
Write-Host "=== Verify MCP configs removed ==="
$paths = @(
    "\\wsl.localhost\Ubuntu\home\lerekie\.hermes\mcp.yaml",
    "C:\Users\lerki\.codex\mcp.yaml",
    "C:\Users\lerki\.gemini\mcp.yaml",
    "C:\Users\lerki\.openclaw\mcp.yaml"
)
foreach ($p in $paths) {
    if (Test-Path $p) {
        Write-Host ("STILL EXISTS: " + $p)
        Get-Content $p
    } else {
        Write-Host ("REMOVED: " + $p)
    }
}

Write-Host ""
Write-Host "=== Check openclaw.json for MCP config ==="
$ocPath = "C:\Users\lerki\.openclaw\openclaw.json"
if (Test-Path $ocPath) {
    $oc = Get-Content $ocPath -Raw | ConvertFrom-Json
    if ($oc.mcp) {
        Write-Host "MCP section still in openclaw.json:"
        $oc.mcp | ConvertTo-Json
    } else {
        Write-Host "MCP section removed from openclaw.json"
    }
}
