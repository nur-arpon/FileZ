# Store slides for FileZ, same recipe as Spaceadom's: 2560x1440, bold headline, one-line
# description with an accent dot, the cropped screenshot as the hero on a radial background.
# Rendered by headless Chrome. Output: out\slide-NN.png
$ErrorActionPreference = 'Stop'
$root   = Split-Path -Parent $MyInvocation.MyCommand.Path
$shots  = Join-Path (Split-Path $root) 'screenshots\cropped'
$font   = 'D:/Claude-Projects/SpaceToggle-V14/src/assets/fonts/outfit-latin-wght-normal.woff2'
$chrome = 'C:\Program Files\Google\Chrome\Application\chrome.exe'
$out    = Join-Path $root 'out'; New-Item -ItemType Directory -Force $out | Out-Null
$tmp    = Join-Path $root 'tmp'; New-Item -ItemType Directory -Force $tmp | Out-Null
$themes = @{
  earthy = @{ bg='radial-gradient(1600px 1000px at 15% 0%, #fbf3e3 0%, #f5ead8 55%, #eadac0 100%)'; text='#201e1d'; soft='#5c554d'; accent='#c67139'; shadow='rgba(90,60,30,.32)'; edge='rgba(90,60,30,.18)' }
  navy   = @{ bg='radial-gradient(1600px 1000px at 15% 0%, #1a2740 0%, #0d141f 60%, #070b12 100%)'; text='#e8e4dc'; soft='#b3bdcc'; accent='#6b8cd6'; shadow='rgba(0,0,0,.6)';    edge='rgba(255,255,255,.10)' }
}
$slides = @(
  @{ n=1; img='01-home-light.png';        theme='earthy'; h='Your Downloads folder, tidy by itself.';                 s='Every file it moved, with Put back on the row. Nothing is ever deleted.' }
  @{ n=2; img='06-wizard-categories.png'; theme='earthy'; h='Three taps of setup. Then forget it exists.';            s='Pick the categories you want and see the folders you will get.' }
  @{ n=3; img='02-rules.png';             theme='earthy'; h='Rules you can actually read.';                            s='By file type, by a word in the name, by the website it came from.' }
  @{ n=4; img='03-history.png';           theme='earthy'; h='Changed your mind? Put it back.';                         s='One file, or everything from today, in one click.' }
  @{ n=5; img='05-home-dark.png';         theme='navy';   h='Light, dark, or follow Windows.';                         s='A quiet note at the bottom of the screen when something is filed.' }
  @{ n=6; img='04-settings.png';          theme='earthy'; h='Every setting is a switch.';                              s='No account, no internet, no AI. Your files never leave your PC.' }
)
foreach ($s in $slides) {
  $t = $themes[$s.theme]
  $src = 'file:///' + ((Join-Path $shots $s.img) -replace '\\','/')
  $html = @"
<!doctype html><html><head><meta charset="utf-8"><style>
@font-face{font-family:Outfit;src:url('file:///$font') format('woff2');font-weight:100 900}
html,body{margin:0;width:2560px;height:1440px;overflow:hidden}
body{background:$($t.bg);font-family:Outfit,'Segoe UI Variable','Segoe UI',sans-serif;color:$($t.text);position:relative}
.h{margin:104px 150px 0;font-size:92px;font-weight:700;letter-spacing:-.02em;line-height:1.06}
.s{margin:26px 154px 0;font-size:42px;font-weight:400;color:$($t.soft)}
.dot{display:inline-block;width:24px;height:24px;border-radius:50%;background:$($t.accent);margin-right:20px;vertical-align:middle;position:relative;top:-5px}
.row{position:absolute;left:150px;right:150px;bottom:90px;height:900px;display:flex;align-items:center;justify-content:center}
.shot{max-height:900px;border-radius:26px;overflow:hidden;box-shadow:0 36px 110px $($t.shadow);border:2px solid $($t.edge);display:flex}
.shot img{max-height:896px;max-width:100%;display:block;object-fit:contain}
</style></head><body>
<div class="h">$($s.h)</div>
<div class="s"><span class="dot"></span>$($s.s)</div>
<div class="row"><div class="shot"><img src="$src"></div></div>
</body></html>
"@
  $htmlPath = Join-Path $tmp ("slide-{0:D2}.html" -f $s.n)
  [IO.File]::WriteAllText($htmlPath, $html, [Text.UTF8Encoding]::new($false))
  $png = Join-Path $out ("slide-{0:D2}.png" -f $s.n)
  & $chrome --headless=new --disable-gpu --hide-scrollbars --allow-file-access-from-files --window-size=2560,1440 --screenshot="$png" ("file:///" + ($htmlPath -replace '\\','/')) 2>$null | Out-Null
  Write-Host ("slide {0}: {1:N0} B" -f $s.n, (Get-Item $png).Length)
}
