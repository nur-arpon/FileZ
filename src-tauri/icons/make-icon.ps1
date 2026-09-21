# Draws the TidyUp icon at 1024 px: Earthy terracotta tile, a cream folder, a sage tick.
# Then `npx tauri icon src-tauri/icons/source.png` produces every size and the Store logos.
Add-Type -AssemblyName System.Drawing
$S = 1024
$bmp = New-Object System.Drawing.Bitmap $S, $S
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = 'AntiAlias'; $g.InterpolationMode = 'HighQualityBicubic'; $g.PixelOffsetMode = 'HighQuality'
$g.Clear([System.Drawing.Color]::Transparent)

function RoundRect([float]$x, [float]$y, [float]$w, [float]$h, [float]$r) {
    $p = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = $r * 2
    $p.AddArc($x, $y, $d, $d, 180, 90); $p.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
    $p.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90); $p.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
    $p.CloseFigure(); $p
}

# tile: terracotta with a subtle warm gradient
$tile = RoundRect 0 0 $S $S 224
$grad = New-Object System.Drawing.Drawing2D.LinearGradientBrush ([System.Drawing.Point]::new(0, 0)), ([System.Drawing.Point]::new($S, $S)), ([System.Drawing.Color]::FromArgb(255, 214, 112, 70)), ([System.Drawing.Color]::FromArgb(255, 184, 84, 48))
$g.FillPath($grad, $tile)

# folder: back tab + front body, cream
$cream = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 255, 250, 241))
$creamDark = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 241, 232, 214))
$g.FillPath($creamDark, (RoundRect 176 262 380 130 44))     # tab
$g.FillPath($creamDark, (RoundRect 176 330 672 440 56))     # back
$g.FillPath($cream, (RoundRect 176 396 672 374 56))         # front

# tick: sage, thick round stroke
$pen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 122, 150, 118)), 74
$pen.StartCap = 'Round'; $pen.EndCap = 'Round'; $pen.LineJoin = 'Round'
$g.DrawLines($pen, [System.Drawing.PointF[]]@([System.Drawing.PointF]::new(392, 578), [System.Drawing.PointF]::new(486, 668), [System.Drawing.PointF]::new(640, 500)))

$g.Dispose()
$out = Join-Path $PSScriptRoot 'source.png'
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
"wrote $out"
