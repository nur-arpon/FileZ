# Draws the FileZ icon at 1024 px: navy tile, three fanned indigo folders with earthy tabs (option I1).
# Then `npx tauri icon src-tauri/icons/source.png` produces every size and the Store logos.
Add-Type -AssemblyName System.Drawing
$S = 1024; $k = 2.0
function RoundRect([float]$x, [float]$y, [float]$w, [float]$h, [float]$r) {
    $x *= $k; $y *= $k; $w *= $k; $h *= $k; $r *= $k
    $p = New-Object System.Drawing.Drawing2D.GraphicsPath; $d = $r * 2
    $p.AddArc($x, $y, $d, $d, 180, 90); $p.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
    $p.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90); $p.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
    $p.CloseFigure(); return $p
}
function C($r, $g, $b) { return [System.Drawing.Color]::FromArgb(255, $r, $g, $b) }
function Fill($g, $color, $path) { $b = New-Object System.Drawing.SolidBrush $color; $g.FillPath($b, $path); $b.Dispose() }
$bmp = New-Object System.Drawing.Bitmap $S, $S
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = 'AntiAlias'; $g.PixelOffsetMode = 'HighQuality'; $g.Clear([System.Drawing.Color]::Transparent)
$grad = New-Object System.Drawing.Drawing2D.LinearGradientBrush ([System.Drawing.Point]::new(0, 0)), ([System.Drawing.Point]::new($S, $S)), (C 22 30 48), (C 11 17 28)
$g.FillPath($grad, (RoundRect 0 0 512 512 112)); $grad.Dispose()
Fill $g (C 44 58 96) (RoundRect 140 140 300 210 26);  Fill $g (C 255 250 241) (RoundRect 140 112 110 56 16)
Fill $g (C 60 79 128) (RoundRect 106 180 300 210 26);  Fill $g (C 142 165 138) (RoundRect 106 152 110 56 16)
Fill $g (C 107 140 214) (RoundRect 72 220 300 210 26); Fill $g (C 214 112 70) (RoundRect 72 192 110 56 16)
Fill $g (C 122 154 224) (RoundRect 72 250 300 180 26)
$g.Dispose()
$out = Join-Path $PSScriptRoot 'source.png'
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose(); "wrote $out"
