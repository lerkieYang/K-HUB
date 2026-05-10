$path = "C:\Users\lerki\.openclaw\openclaw.json"
$raw = Get-Content $path -Raw
$obj = $raw | ConvertFrom-Json
$obj.PSObject.Properties.Remove('mcp')
$obj | ConvertTo-Json -Depth 10 | Set-Content $path -Encoding UTF8
Write-Host "MCP section removed from openclaw.json"
