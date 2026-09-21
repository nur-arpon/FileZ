# Draws four TidyUp icon candidates at 512 px and a side-by-side sheet for choosing.
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
function Tile($g, $c1, $c2) { $grad = New-Object System.Drawing.Drawing2D.LinearGradientBrush ([System.Drawing.Point]::new(0, 0)), ([System.Drawing.Point]::new($S, $S)), $c1, $c2; $g.FillPath($grad, (RoundRect 0 0 $S $S 112)); $grad.Dispose() }
function Tick($g, $color, $w, $pts) { $pen = New-Object System.Drawing.Pen $color, ([float]$w); $pen.StartCap = 'Round'; $pen.EndCap = 'Round'; $pen.LineJoin = 'Round'; $g.DrawLines($pen, [System.Drawing.PointF[]]$pts); $pen.Dispose() }
function Folder($g, $back, $front) { Fill $g $back (RoundRect 88 131 190 65 22); Fill $g $back (RoundRect 88 165 336 220 28); Fill $g $front (RoundRect 88 198 336 187 28) }
function Canvas { $bmp = New-Object System.Drawing.Bitmap $S, $S; $g = [System.Drawing.Graphics]::FromImage($bmp); $g.SmoothingMode = 'AntiAlias'; $g.PixelOffsetMode = 'HighQuality'; $g.Clear([System.Drawing.Color]::Transparent); return $bmp, $g }
$cream = C 255 250 241; $creamDark = C 241 232 214; $terra = C 200 98 58; $terraDark = C 176 78 44; $sage = C 122 150 118; $indigo = C 107 140 214
$tickPts = @((P 196 289), (P 243 334), (P 320 250))
$out = [ordered]@{}

$bmp, $g = Canvas; Tile $g (C 214 112 70) (C 184 84 48); Folder $g $creamDark $cream; Tick $g $sage 37 $tickPts; $g.Dispose(); $out['A'] = $bmp
$bmp, $g = Canvas; Tile $g (C 246 240 228) (C 236 226 206); Folder $g $terraDark $terra; Tick $g $cream 37 $tickPts; $g.Dispose(); $out['B'] = $bmp
$bmp, $g = Canvas; Tile $g (C 19 26 41) (C 13 20 31); Folder $g (C 60 79 128) $indigo; Tick $g $cream 37 $tickPts; $g.Dispose(); $out['C'] = $bmp
$bmp, $g = Canvas; Tile $g (C 214 112 70) (C 184 84 48)
Fill $g (C 232 200 182) (RoundRect 150 110 270 190 26); Fill $g $creamDark (RoundRect 122 160 270 190 26); Fill $g $cream (RoundRect 94 210 270 190 26)
Tick $g $sage 30 @((P 170 300), (P 210 338), (P 290 262)); $g.Dispose(); $out['D'] = $bmp

$sheet = New-Object System.Drawing.Bitmap 1180, 400; $g = [System.Drawing.Graphics]::FromImage($sheet); $g.SmoothingMode = 'AntiAlias'; $g.InterpolationMode = 'HighQualityBicubic'
$g.Clear((C 238 238 238)); $font = [System.Drawing.Font]::new('Segoe UI', [float]22, [System.Drawing.FontStyle]::Bold); $i = 0
foreach ($k in @('A', 'B', 'C', 'D')) {
    $x = 30 + $i * 285; $img = $out[$k]
    $g.DrawImage($img, [int]$x, 40, 256, 256); $g.DrawImage($img, [int]($x + 100), 320, 48, 48); $g.DrawImage($img, [int]($x + 170), 330, 24, 24)
    $g.DrawString($k, $font, [System.Drawing.Brushes]::Black, [float]$x, [float]320)
    $img.Save((Join-Path $PSScriptRoot "option-$k.png"), [System.Drawing.Imaging.ImageFormat]::Png); $i++
}
$g.Dispose(); $sheet.Save((Join-Path $PSScriptRoot 'icon-options.png'), [System.Drawing.Imaging.ImageFormat]::Png); "wrote icon-options.png"
