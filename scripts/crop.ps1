param(
  [Parameter(Mandatory=$true)][string]$Sheet,
  [int]$X0, [int]$Y0, [int]$ColPitch, [int]$RowPitch, [int]$CropW, [int]$CropH,
  [Parameter(Mandatory=$true)][string]$OutDir,
  # each entry "slug:col:row"
  [Parameter(Mandatory=$true)][string[]]$Cells
)
Add-Type -AssemblyName System.Drawing
if (-not (Test-Path $OutDir)) { New-Item -ItemType Directory -Force $OutDir | Out-Null }
$img = [System.Drawing.Bitmap]::FromFile($Sheet)
foreach ($c in $Cells) {
  $p = $c.Split(':'); $slug = $p[0]; [int]$col = $p[1]; [int]$row = $p[2]
  $x = $X0 + $col * $ColPitch
  $y = $Y0 + $row * $RowPitch
  $w = [Math]::Min($CropW, $img.Width - $x)
  $h = [Math]::Min($CropH, $img.Height - $y)
  if ($x -lt 0 -or $y -lt 0 -or $w -le 0 -or $h -le 0) { Write-Host "skip $slug (out of bounds $x,$y)"; continue }
  $rect = New-Object System.Drawing.Rectangle($x, $y, $w, $h)
  $crop = $img.Clone($rect, $img.PixelFormat)
  $crop.Save((Join-Path $OutDir ($slug + '.png')), [System.Drawing.Imaging.ImageFormat]::Png)
  $crop.Dispose()
  Write-Host "$slug  <=  ($x,$y  ${w}x${h})"
}
$img.Dispose()
