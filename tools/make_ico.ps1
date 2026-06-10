# Builds a multi-size ICO (PNG-compressed entries) from a source PNG.
param(
  [Parameter(Mandatory = $true)][string]$Source,
  [Parameter(Mandatory = $true)][string]$Destination
)

Add-Type -AssemblyName System.Drawing

$sizes = 256, 128, 64, 48, 32, 24, 16
$src = [System.Drawing.Image]::FromFile($Source)
try {
  $blobs = foreach ($size in $sizes) {
    $bmp = New-Object System.Drawing.Bitmap($size, $size)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.DrawImage($src, 0, 0, $size, $size)
    $g.Dispose()
    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    , $ms.ToArray()
  }
}
finally {
  $src.Dispose()
}

$out = New-Object System.IO.MemoryStream
$writer = New-Object System.IO.BinaryWriter($out)
$writer.Write([UInt16]0)             # reserved
$writer.Write([UInt16]1)             # type: icon
$writer.Write([UInt16]$sizes.Count)  # image count

$offset = 6 + 16 * $sizes.Count
for ($i = 0; $i -lt $sizes.Count; $i++) {
  $size = $sizes[$i]
  $dim = if ($size -ge 256) { 0 } else { $size }
  $writer.Write([Byte]$dim)          # width (0 = 256)
  $writer.Write([Byte]$dim)          # height
  $writer.Write([Byte]0)             # palette colors
  $writer.Write([Byte]0)             # reserved
  $writer.Write([UInt16]1)           # color planes
  $writer.Write([UInt16]32)          # bits per pixel
  $writer.Write([UInt32]$blobs[$i].Length)
  $writer.Write([UInt32]$offset)
  $offset += $blobs[$i].Length
}
foreach ($blob in $blobs) { $writer.Write($blob) }
$writer.Flush()
[System.IO.File]::WriteAllBytes($Destination, $out.ToArray())
$writer.Dispose()
Write-Output "Wrote $Destination ($((Get-Item $Destination).Length) bytes)"
