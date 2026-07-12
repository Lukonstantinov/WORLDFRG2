param(
  [string]$SrcDir = "C:\Users\Luka\Desktop\Fishes",
  [string]$FishDir,                 # public/fish
  [string]$VerifyOut,               # montage of crop vs current plate
  [switch]$Apply
)
Add-Type -AssemblyName System.Drawing

# file-code -> slug, in the gallery's continuous order (verified by distinctive fish)
$map = @(
  # Boreal
  "022519=arctic-grayling","022538=broad-whitefish","022543=frostscale-grayling","022548=redflank-char",
  "022552=silverfin-trout","022559=silverpike-taimen","022604=boreal-burbot","022608=goldeye-ide",
  "022612=northern-pike","022615=blackfin-char","022619=silverleap-salmon","022625=thornback-ruffe",
  # Temperate 17-24
  "022648=greatmaw-catfish","022652=marsh-carp","022657=mussel-bitterling","022701=redwing-rudd",
  "022705=reedwater-zander","022711=silt-sturgeon","022714=silverback-eel","022719=tidewater-shad",
  # Tropical 1-8
  "022723=cataract-loach","022728=emberscale-tetra","022733=golden-pacu","022739=redhook-myleus",
  "022743=sabertooth-payara","022747=silver-bocachico","022751=brackish-snook","022755=delta-mullet",
  # Lakes 1-12
  "022806=azure-mbuna","022810=goldshoal-tilapia","022815=lakeshore-pupfish","022819=mosaic-cichlid",
  "022824=reed-tench","022828=brine-shrimp","022833=glass-cisco","022838=pallid-omul",
  "022842=silver-nerpa","022846=abyss-sculpin","022851=graven-laketrout","022856=marsh-lungfish",
  # Temperate 1-16
  "023005=marble-trout","023010=ribbon-dace","023014=stonecling-bullhead","023017=torrent-loach",
  "023022=barbeled-gudgeon","023025=blueback-vimba","023030=bronze-chub","023033=curveblade-sabrefish",
  "023038=goldvein-barbel","023041=gravel-nase","023046=marbled-perch","023050=rustscale-roach",
  "023054=sailback-asp","023058=silverstreak-bleak","023102=whiskered-wels","023106=broadscale-bream"
)

$entries = @()
foreach ($m in $map) {
  $p = $m.Split('='); $code = $p[0]; $slug = $p[1]
  $file = Get-ChildItem $SrcDir -Filter ("*" + $code + "*.png") | Select-Object -First 1
  if ($null -eq $file) { Write-Host "MISSING SRC $code"; continue }
  $entries += [pscustomobject]@{ code=$code; slug=$slug; src=$file.FullName }
}

if ($VerifyOut) {
  [int]$cols = 6; [int]$cw = 150; [int]$ch = 92; [int]$cap = 20; [int]$gap = 6
  [int]$cellW = $cw * 2 + 8; [int]$cellH = $ch + $cap
  [int]$rows = [Math]::Ceiling($entries.Count / $cols)
  $mont = New-Object System.Drawing.Bitmap(($cols * ($cellW + $gap) + $gap), ($rows * ($cellH + $gap) + $gap))
  $g = [System.Drawing.Graphics]::FromImage($mont); $g.Clear([System.Drawing.Color]::FromArgb(10,15,22))
  $font = New-Object System.Drawing.Font("Segoe UI", 8)
  $wb = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
  $i = 0
  foreach ($e in $entries) {
    $col = $i % $cols; $row = [int]($i / $cols)
    $px = $gap + $col * ($cellW + $gap); $py = $gap + $row * ($cellH + $gap)
    $ci = [System.Drawing.Image]::FromFile($e.src); $g.DrawImage($ci, $px, $py, $cw, $ch); $ci.Dispose()
    $rp = Join-Path $FishDir ($e.slug + '.png')
    if (Test-Path $rp) { $ri = [System.Drawing.Image]::FromFile($rp); $g.DrawImage($ri, $px + $cw + 8, $py, $cw, $ch); $ri.Dispose() }
    $g.DrawString($e.slug, $font, $wb, $px + 2, $py + $ch + 2)
    $i++
  }
  $g.Dispose(); $mont.Save($VerifyOut, [System.Drawing.Imaging.ImageFormat]::Png); $mont.Dispose()
  Write-Host "verify -> $VerifyOut ($($entries.Count))"
}

if ($Apply) {
  foreach ($e in $entries) { Copy-Item $e.src (Join-Path $FishDir ($e.slug + '.png')) -Force }
  Write-Host "applied $($entries.Count) crops to $FishDir"
}
