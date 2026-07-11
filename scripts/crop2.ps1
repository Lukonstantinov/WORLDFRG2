param(
  [Parameter(Mandatory=$true)][string]$Sheet,
  [int]$X0, [int]$Y0, [int]$ColPitch, [int]$RowPitch, [int]$CropW, [int]$CropH,
  [Parameter(Mandatory=$true)][string]$OutDir,
  [Parameter(Mandatory=$true)][string[]]$Cells,   # "slug:col:row"
  [string]$Montage = ""                            # optional preview grid path
)
Add-Type -AssemblyName System.Drawing
if (-not (Test-Path $OutDir)) { New-Item -ItemType Directory -Force $OutDir | Out-Null }
$img = [System.Drawing.Bitmap]::FromFile($Sheet)

$parsed = @()
foreach ($c in $Cells) { $p = $c.Split(':'); $parsed += [pscustomobject]@{ slug=$p[0]; col=[int]$p[1]; row=[int]$p[2] } }

foreach ($e in $parsed) {
  $x = $X0 + $e.col * $ColPitch
  $y = $Y0 + $e.row * $RowPitch
  $w = [Math]::Min($CropW, $img.Width - $x)
  $h = [Math]::Min($CropH, $img.Height - $y)
  if ($x -lt 0 -or $y -lt 0 -or $w -le 0 -or $h -le 0) { Write-Host "skip $($e.slug)"; continue }
  $rect = New-Object System.Drawing.Rectangle($x, $y, $w, $h)
  $crop = $img.Clone($rect, $img.PixelFormat)
  $crop.Save((Join-Path $OutDir ($e.slug + '.png')), [System.Drawing.Imaging.ImageFormat]::Png)
  $crop.Dispose()
}
Write-Host "cropped $($parsed.Count) -> $OutDir"

if ($Montage -ne "") {
  [int]$maxCol = 0; [int]$maxRow = 0
  foreach ($e in $parsed) { if ($e.col -gt $maxCol) { $maxCol = $e.col }; if ($e.row -gt $maxRow) { $maxRow = $e.row } }
  [int]$gap = 6
  [int]$mw = ($maxCol + 1) * ($CropW + $gap) + $gap
  [int]$mh = ($maxRow + 1) * ($CropH + $gap) + $gap
  $mont = New-Object System.Drawing.Bitmap($mw, $mh)
  $g = [System.Drawing.Graphics]::FromImage($mont)
  $g.Clear([System.Drawing.Color]::FromArgb(20, 28, 40))
  foreach ($e in $parsed) {
    $f = Join-Path $OutDir ($e.slug + '.png')
    if (Test-Path $f) {
      $ci = [System.Drawing.Image]::FromFile($f)
      $px = $gap + $e.col * ($CropW + $gap)
      $py = $gap + $e.row * ($CropH + $gap)
      $g.DrawImage($ci, $px, $py, $CropW, $CropH)
      $ci.Dispose()
    }
  }
  $g.Dispose()
  $mont.Save($Montage, [System.Drawing.Imaging.ImageFormat]::Png)
  $mont.Dispose()
  Write-Host "montage -> $Montage"
}
$img.Dispose()
