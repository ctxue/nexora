param(
  [string]$OutputDirectory = (Join-Path $PSScriptRoot "..\assets")
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

function New-HaloPng {
  param(
    [int]$Size,
    [string]$Path
  )

  $bitmap = New-Object System.Drawing.Bitmap(
    $Size,
    $Size,
    [System.Drawing.Imaging.PixelFormat]::Format32bppArgb
  )
  $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
  $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
  $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
  $graphics.Clear([System.Drawing.Color]::Transparent)
  $scale = $Size / 256.0

  # Two-color identity: a warm ivory island body with deep-ink eyes.
  $islandColor = [System.Drawing.Color]::FromArgb(255, 250, 245, 234)
  $ink = [System.Drawing.Color]::FromArgb(255, 24, 24, 29)
  $islandBrush = New-Object System.Drawing.SolidBrush($islandColor)
  $inkBrush = New-Object System.Drawing.SolidBrush($ink)

  $body = New-Object System.Drawing.Drawing2D.GraphicsPath
  $body.StartFigure()
  $body.AddBezier(
    (54 * $scale), (196 * $scale),
    (45 * $scale), (142 * $scale),
    (58 * $scale), (91 * $scale),
    (91 * $scale), (68 * $scale)
  )
  $body.AddBezier(
    (91 * $scale), (68 * $scale),
    (110 * $scale), (54 * $scale),
    (146 * $scale), (54 * $scale),
    (165 * $scale), (68 * $scale)
  )
  $body.AddBezier(
    (165 * $scale), (68 * $scale),
    (198 * $scale), (91 * $scale),
    (211 * $scale), (142 * $scale),
    (202 * $scale), (196 * $scale)
  )
  $body.AddBezier(
    (202 * $scale), (196 * $scale),
    (188 * $scale), (213 * $scale),
    (68 * $scale), (213 * $scale),
    (54 * $scale), (196 * $scale)
  )
  $body.CloseFigure()
  $graphics.FillPath($islandBrush, $body)

  # Two oversized eyes stay legible down to the 16px tray size.
  foreach ($eyeX in @(91, 139)) {
    $graphics.FillEllipse(
      $inkBrush,
      ((($eyeX - 11) * $scale)),
      ((108 * $scale)),
      (22 * $scale),
      (38 * $scale)
    )
    $graphics.FillEllipse(
      $islandBrush,
      ((($eyeX - 3.5) * $scale)),
      ((113 * $scale)),
      (7 * $scale),
      (10 * $scale)
    )
  }

  # A tiny island crest keeps the silhouette playful and asymmetric.
  $graphics.FillEllipse(
    $inkBrush,
    (119 * $scale),
    (51 * $scale),
    (18 * $scale),
    (10 * $scale)
  )

  $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
  $inkBrush.Dispose()
  $islandBrush.Dispose()
  $body.Dispose()
  $graphics.Dispose()
  $bitmap.Dispose()
}

$assetDirectory = [System.IO.Path]::GetFullPath($OutputDirectory)
[System.IO.Directory]::CreateDirectory($assetDirectory) | Out-Null
$logoPath = Join-Path $assetDirectory "nexora-halo-logo.png"
New-HaloPng -Size 512 -Path $logoPath

$sizes = @(16, 32, 48, 64, 128, 256)
$frames = @()
foreach ($size in $sizes) {
  $path = Join-Path $assetDirectory ("nexora-halo-{0}.png" -f $size)
  New-HaloPng -Size $size -Path $path
  $frames += [pscustomobject]@{
    Size = $size
    Path = $path
    Bytes = [System.IO.File]::ReadAllBytes($path)
  }
}

$iconPath = Join-Path $assetDirectory "nexora-halo.ico"
$stream = [System.IO.File]::Create($iconPath)
$writer = New-Object System.IO.BinaryWriter($stream)
try {
  $writer.Write([uint16]0)
  $writer.Write([uint16]1)
  $writer.Write([uint16]$frames.Count)
  $offset = 6 + (16 * $frames.Count)
  foreach ($frame in $frames) {
    $dimension = if ($frame.Size -eq 256) { 0 } else { $frame.Size }
    $writer.Write([byte]$dimension)
    $writer.Write([byte]$dimension)
    $writer.Write([byte]0)
    $writer.Write([byte]0)
    $writer.Write([uint16]1)
    $writer.Write([uint16]32)
    $writer.Write([uint32]$frame.Bytes.Length)
    $writer.Write([uint32]$offset)
    $offset += $frame.Bytes.Length
  }
  foreach ($frame in $frames) {
    $writer.Write($frame.Bytes)
  }
} finally {
  $writer.Dispose()
  $stream.Dispose()
}

foreach ($frame in $frames) {
  [System.IO.File]::Delete($frame.Path)
}

Write-Output $logoPath
Write-Output $iconPath
