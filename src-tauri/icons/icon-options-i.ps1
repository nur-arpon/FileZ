# Option I (three fanned folders, tabbed) in five tab palettes, plus a sheet.
Add-Type -AssemblyName System.Drawing
$S = 512
function RoundRect([float]$x, [float]$y, [float]$w, [float]$h, [float]$r) {
    $p = New-Object System.Drawing.Drawing2D.GraphicsPath; $d = $r * 2
    $p.AddArc($x, $y, $d, $d, 180, 90); $p.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
    $p.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90); $p.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
    $p.CloseFigure(); return $p
}
function C($r, $g, $b) { return [System.Drawing.Color]::FromArgb(255, $r, $g, $b) }
function Fill($g, $color, $path) { $b = New-Object System.Drawing.SolidBrush $color; $g.FillPath($b, $path); $b.Dispose() }
function Tile($g) { $grad = New-Object System.Drawing.Drawing2D.LinearGradientBrush ([System.Drawing.Point]::new(0, 0)), ([System.Drawing.Point]::new($S, $S)), (C 22 30 48), (C 11 17 28); $g.FillPath($grad, (RoundRect 0 0 $S $S 112)); $grad.Dispose() }
function Canvas { $bmp = New-Object System.Drawing.Bitmap $S, $S; $g = [System.Drawing.Graphics]::FromImage($bmp); $g.SmoothingMode = 'AntiAlias'; $g.PixelOffsetMode = 'HighQuality'; $g.Clear([System.Drawing.Color]::Transparent); return $bmp, $g }
function Stack($g, $tabBack, $tabMid, $tabFront) {
    Fill $g (C 44 58 96) (RoundRect 140 140 300 210 26); Fill $g $tabBack (RoundRect 140 112 110 56 16)
    Fill $g (C 60 79 128) (RoundRect 106 180 300 210 26); Fill $g $tabMid (RoundRect 106 152 110 56 16)
    Fill $g (C 107 140 214) (RoundRect 72 220 300 210 26); Fill $g $tabFront (RoundRect 72 192 110 56 16)
    Fill $g (C 122 154 224) (RoundRect 72 250 300 180 26)
}
$palettes = [ordered]@{
    'I1' = @((C 255 250 241), (C 142 165 138), (C 214 112 70))    # earthy, as before
    'I2' = @((C 183 200 236), (C 140 165 224), (C 226 232 245))   # all indigo tints, one family
    'I3' = @((C 255 214 102), (C 255 138 101), (C 255 99 132))    # sunset: amber, coral, pink
    'I4' = @((C 94 234 212), (C 96 165 250), (C 192 132 252))     # cool: teal, sky, violet
    'I5' = @((C 255 255 255), (C 255 255 255), (C 255 255 255))   # plain white tabs
    'I6' = @((C 250 204 21), (C 74 222 128), (C 251 113 133))     # bright: yellow, green, rose
}
$out = [ordered]@{}
foreach ($k in $palettes.Keys) { $bmp, $g = Canvas; Tile $g; $p = $palettes[$k]; Stack $g $p[0] $p[1] $p[2]; $g.Dispose(); $out[$k] = $bmp }

$sheet = New-Object System.Drawing.Bitmap 1780, 400; $g = [System.Drawing.Graphics]::FromImage($sheet); $g.SmoothingMode = 'AntiAlias'; $g.InterpolationMode = 'HighQualityBicubic'
$g.Clear((C 238 238 238)); $font = [System.Drawing.Font]::new('Segoe UI', [float]22, [System.Drawing.FontStyle]::Bold); $i = 0
foreach ($k in $out.Keys) {
    $x = 30 + $i * 290; $img = $out[$k]
    $g.DrawImage($img, [int]$x, 40, 256, 256); $g.DrawImage($img, [int]($x + 110), 320, 48, 48); $g.DrawImage($img, [int]($x + 180), 330, 24, 24)
    $g.DrawString($k, $font, [System.Drawing.Brushes]::Black, [float]$x, [float]320)
    $img.Save((Join-Path $PSScriptRoot "option-$k.png"), [System.Drawing.Imaging.ImageFormat]::Png); $i++
}
$g.Dispose(); $sheet.Save((Join-Path $PSScriptRoot 'icon-options-i.png'), [System.Drawing.Imaging.ImageFormat]::Png); "wrote icon-options-i.png"
