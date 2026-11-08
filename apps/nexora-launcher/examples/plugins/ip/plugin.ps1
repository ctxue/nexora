$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

while ($true) {
    $line = [Console]::In.ReadLine()
    if ($null -eq $line) { break }
    if ([string]::IsNullOrWhiteSpace($line)) { continue }

    try {
        $message = $line | ConvertFrom-Json
    } catch {
        continue
    }

    if ($message.method -ne "search") { continue }

    $query = ""
    if ($null -ne $message.params -and $null -ne $message.params.query) {
        $query = [string]$message.params.query
    }
    $query = $query.Trim()

    if ([string]::IsNullOrEmpty($query)) {
        $item = [ordered]@{
            id       = "ip:home"
            title    = "IP lookup"
            subtitle = "Type an address, e.g. ip 8.8.8.8"
            icon     = @{ type = "glyph"; value = "globe" }
            action   = @{ type = "openUrl"; url = "https://ipinfo.io" }
        }
    } else {
        $encoded = [uri]::EscapeDataString($query)
        $item = [ordered]@{
            id       = "ip:$query"
            title    = $query
            subtitle = "Open this address on ipinfo.io"
            icon     = @{ type = "glyph"; value = "globe" }
            action   = @{ type = "openUrl"; url = "https://ipinfo.io/$encoded" }
        }
    }

    $itemJson = $item | ConvertTo-Json -Compress -Depth 6
    $response = '{"jsonrpc":"2.0","id":' + $message.id + ',"result":{"items":[' + $itemJson + ']}}'
    [Console]::Out.WriteLine($response)
    [Console]::Out.Flush()
}
