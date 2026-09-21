# Four navy/indigo icon candidates without a tick, at 512 px, plus a comparison sheet.
Add-Type -AssemblyName System.Drawing
$S = 512
function RoundRect([float]$x, [float]$y, [float]$w, [float]$h, [float]$r) {
    $p = New-Object System.Drawing.Drawing2D.GraphicsPath; $d = $r * 2
    $p.AddArc($x, $y, $d, $d, 180, 90); $p.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
    $p.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90); $p.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
    $p.CloseFigure(); return $p
}
function C($r, $g, $b) { return [System.Drawing.Color]::FromArgb(255, $r, $g, $b) }
function P($x, $y) { return [System.Drawing.PointF]::new($x, $y) }
function Fill($g, $color, $path) { $b = New-Object System.Drawing.SolidBrush $color; $g.FillPath($b, $path); $b.Dispose() }
function Tile($g) { $grad = New-Object System.Drawing.Drawing2D.LinearGradientBrush ([System.Drawing.Point]::new(0, 0)), ([System.Drawing.Point]::new($S, $S)), (C 22 30 48), (C 11 17 28); $g.FillPath($grad, (RoundRect 0 0 $S $S 112)); $grad.Dispose() }
function Stroke($g, $color, $w, $pts) { $pen = New-Object System.Drawing.Pen $color, ([float]$w); $pen.StartCap = 'Round'; $pen.EndCap = 'Round'; $pen.LineJoin = 'Round'; $g.DrawLines($pen, [System.Drawing.PointF[]]$pts); $pen.Dispose() }
function Canvas { $bmp = New-Object System.Drawing.Bitmap $S, $S; $g = [System.Drawing.Graphics]::FromImage($bmp); $g.SmoothingMode = 'AntiAlias'; $g.PixelOffsetMode = 'HighQuality'; $g.Clear([System.Drawing.Color]::Transparent); return $bmp, $g }
$indigo = C 107 140 214; $indigoDark = C 60 79 128; $indigoPale = C 183 200 236; $cream = C 255 250 241; $terra = C 214 112 70; $sage = C 142 165 138
$out = [ordered]@{}

# E: folder with three coloured category tabs on top (terracotta, sage, cream)
$bmp, $g = Canvas; Tile $g
Fill $g $terra (RoundRect 96 128 110 70 18); Fill $g $sage (RoundRect 216 128 110 70 18); Fill $g $cream (RoundRect 336 128 80 70 18)
Fill $g $indigoDark (RoundRect 88 168 336 216 28); Fill $g $indigo (RoundRect 88 200 336 184 28)
$g.Dispose(); $out['E'] = $bmp

# F: four pigeonholes (2x2 grid) with one slot holding a cream file
$bmp, $g = Canvas; Tile $g
Fill $g $indigoDark (RoundRect 92 92 328 328 40)
foreach ($xy in @(@(116, 116), @(268, 116), @(116, 268), @(268, 268))) { Fill $g (C 16 23 38) (RoundRect $xy[0] $xy[1] 128 128 22) }
Fill $g $indigo (RoundRect 116 116 128 128 22); Fill $g $cream (RoundRect 146 140 68 84 10); Stroke $g $indigoPale 8 @((P 160 166), (P 200 166)); Stroke $g $indigoPale 8 @((P 160 186), (P 200 186)); Stroke $g $indigoPale 8 @((P 160 206), (P 186 206))
$g.Dispose(); $out['F'] = $bmp

# G: a file dropping into an open folder (arrow), indigo folder, cream file
$bmp, $g = Canvas; Tile $g
Fill $g $cream (RoundRect 208 70 96 120 14); Stroke $g $indigoPale 8 @((P 228 104), (P 284 104)); Stroke $g $indigoPale 8 @((P 228 128), (P 284 128)); Stroke $g $indigoPale 8 @((P 228 152), (P 266 152))
Fill $g $indigoDark (RoundRect 88 214 336 190 28); Fill $g $indigo (RoundRect 88 250 336 154 28)
Stroke $g $terra 30 @((P 256 196), (P 256 300)); Stroke $g $terra 30 @((P 214 262), (P 256 304), (P 298 262))
$g.Dispose(); $out['G'] = $bmp

# H: three folders fanned back to front, each with its own coloured tab (E + H, no dot)
$bmp, $g = Canvas; Tile $g
Fill $g (C 44 58 96) (RoundRect 150 150 300 200 26); Fill $g $cream (RoundRect 150 118 120 60 18)
Fill $g $indigoDark (RoundRect 112 188 300 200 26); Fill $g $sage (RoundRect 112 156 120 60 18)
Fill $g $indigo (RoundRect 74 226 300 200 26); Fill $g $terra (RoundRect 74 194 120 60 18)
$g.Dispose(); $out['H'] = $bmp

# I: same, but tabs sit flush and the fronts overlap tighter so it reads as one stack at 16 px
$bmp, $g = Canvas; Tile $g
Fill $g (C 44 58 96) (RoundRect 140 140 300 210 26); Fill $g $cream (RoundRect 140 112 110 56 16)
Fill $g $indigoDark (RoundRect 106 180 300 210 26); Fill $g $sage (RoundRect 106 152 110 56 16)
Fill $g $indigo (RoundRect 72 220 300 210 26); Fill $g $terra (RoundRect 72 192 110 56 16)
Fill $g (C 122 154 224) (RoundRect 72 250 300 180 26)
$g.Dispose(); $out['I'] = $bmp

$sheet = New-Object System.Drawing.Bitmap 1180, 400; $g = [System.Drawing.Graphics]::FromImage($sheet); $g.SmoothingMode = 'AntiAlias'; $g.InterpolationMode = 'HighQualityBicubic'
$g.Clear((C 238 238 238)); $font = [System.Drawing.Font]::new('Segoe UI', [float]22, [System.Drawing.FontStyle]::Bold); $i = 0
foreach ($k in @('E', 'H', 'I', 'F')) {
    $x = 30 + $i * 285; $img = $out[$k]
    $g.DrawImage($img, [int]$x, 40, 256, 256); $g.DrawImage($img, [int]($x + 100), 320, 48, 48); $g.DrawImage($img, [int]($x + 170), 330, 24, 24)
    $g.DrawString($k, $font, [System.Drawing.Brushes]::Black, [float]$x, [float]320)
    $img.Save((Join-Path $PSScriptRoot "option-$k.png"), [System.Drawing.Imaging.ImageFormat]::Png); $i++
}
$g.Dispose(); $sheet.Save((Join-Path $PSScriptRoot 'icon-options-c.png'), [System.Drawing.Imaging.ImageFormat]::Png); "wrote icon-options-c.png"
