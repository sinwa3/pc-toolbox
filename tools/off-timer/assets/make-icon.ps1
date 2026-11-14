# Draws the off-timer icon (power symbol + clock hand) with System.Drawing
# and writes icon.ico (16-256 px, PNG entries), icon-256.png and icon-64.rgba (window icon).
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File make-icon.ps1
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing
$here = Split-Path -Parent $MyInvocation.MyCommand.Path

function Draw-Icon([int]$size) {
    $s = $size / 256.0
    $bmp = New-Object System.Drawing.Bitmap $size, $size, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.Clear([System.Drawing.Color]::Transparent)

    # Rounded square background, Windows-blue gradient
    $m = 8 * $s; $w = $size - 2 * $m; $r = 56 * $s
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $path.AddArc($m, $m, 2 * $r, 2 * $r, 180, 90)
    $path.AddArc($m + $w - 2 * $r, $m, 2 * $r, 2 * $r, 270, 90)
    $path.AddArc($m + $w - 2 * $r, $m + $w - 2 * $r, 2 * $r, 2 * $r, 0, 90)
    $path.AddArc($m, $m + $w - 2 * $r, 2 * $r, 2 * $r, 90, 90)
    $path.CloseFigure()
    $top = [System.Drawing.Color]::FromArgb(255, 0x2B, 0x8C, 0xE0)
    $bottom = [System.Drawing.Color]::FromArgb(255, 0x00, 0x52, 0x9E)
    $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush (New-Object System.Drawing.PointF 0, $m), (New-Object System.Drawing.PointF 0, ($m + $w)), $top, $bottom
    $g.FillPath($brush, $path)

    $white = [System.Drawing.Color]::White
    $cx = 128 * $s; $cy = 138 * $s; $rad = 70 * $s

    # Power ring with a gap at the top
    $pen = New-Object System.Drawing.Pen $white, (20 * $s)
    $pen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $pen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $g.DrawArc($pen, $cx - $rad, $cy - $rad, 2 * $rad, 2 * $rad, 302, 296)

    # Power line = minute hand at 12
    $g.DrawLine($pen, $cx, 46 * $s, $cx, $cy)

    # Hour hand toward 4 o'clock
    $hand = New-Object System.Drawing.Pen $white, (16 * $s)
    $hand.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $hand.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $a = 30 * [Math]::PI / 180
    $g.DrawLine($hand, $cx, $cy, $cx + 40 * $s * [Math]::Cos($a), $cy + 40 * $s * [Math]::Sin($a))

    $g.Dispose()
    return $bmp
}

function To-Png([System.Drawing.Bitmap]$bmp) {
    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    return , $ms.ToArray()
}

# Classic 32-bit DIB entry (BITMAPINFOHEADER + bottom-up BGRA + empty AND mask).
# Older APIs only read PNG entries at 256 px, so smaller sizes use this.
function To-Dib([System.Drawing.Bitmap]$bmp) {
    $n = $bmp.Width
    $ms = New-Object System.IO.MemoryStream
    $bw = New-Object System.IO.BinaryWriter $ms
    $bw.Write([UInt32]40); $bw.Write([Int32]$n); $bw.Write([Int32]($n * 2))
    $bw.Write([UInt16]1); $bw.Write([UInt16]32); $bw.Write([UInt32]0)
    $maskRow = [int]([Math]::Ceiling($n / 32.0) * 4)
    $bw.Write([UInt32]($n * $n * 4 + $maskRow * $n))
    $bw.Write([Int32]0); $bw.Write([Int32]0); $bw.Write([UInt32]0); $bw.Write([UInt32]0)
    for ($y = $n - 1; $y -ge 0; $y--) {
        for ($x = 0; $x -lt $n; $x++) {
            $c = $bmp.GetPixel($x, $y)
            $bw.Write([byte]$c.B); $bw.Write([byte]$c.G); $bw.Write([byte]$c.R); $bw.Write([byte]$c.A)
        }
    }
    $bw.Write((New-Object byte[] ($maskRow * $n)))
    $bw.Flush()
    return , $ms.ToArray()
}

# icon.ico: DIB entries up to 128 px, PNG entry at 256 px
$sizes = 16, 20, 24, 32, 40, 48, 64, 128, 256
$pngs = @()
foreach ($sz in $sizes) {
    $b = Draw-Icon $sz
    if ($sz -ge 256) { $pngs += , (To-Png $b) } else { $pngs += , (To-Dib $b) }
}
$ms = New-Object System.IO.MemoryStream
$bw = New-Object System.IO.BinaryWriter $ms
$bw.Write([UInt16]0); $bw.Write([UInt16]1); $bw.Write([UInt16]$sizes.Count)
$offset = 6 + 16 * $sizes.Count
for ($i = 0; $i -lt $sizes.Count; $i++) {
    $sz = $sizes[$i]; $d = $pngs[$i]
    $dim = if ($sz -ge 256) { 0 } else { $sz }
    $bw.Write([byte]$dim); $bw.Write([byte]$dim); $bw.Write([byte]0); $bw.Write([byte]0)
    $bw.Write([UInt16]1); $bw.Write([UInt16]32)
    $bw.Write([UInt32]$d.Length); $bw.Write([UInt32]$offset)
    $offset += $d.Length
}
foreach ($d in $pngs) { $bw.Write($d) }
$bw.Flush()
[IO.File]::WriteAllBytes((Join-Path $here "icon.ico"), $ms.ToArray())

# Preview PNG
[IO.File]::WriteAllBytes((Join-Path $here "icon-256.png"), (To-Png (Draw-Icon 256)))

# Raw RGBA 64x64 for the window/taskbar icon (eframe IconData)
$b = Draw-Icon 64
$raw = New-Object byte[] (64 * 64 * 4)
for ($y = 0; $y -lt 64; $y++) {
    for ($x = 0; $x -lt 64; $x++) {
        $c = $b.GetPixel($x, $y); $o = ($y * 64 + $x) * 4
        $raw[$o] = $c.R; $raw[$o + 1] = $c.G; $raw[$o + 2] = $c.B; $raw[$o + 3] = $c.A
    }
}
[IO.File]::WriteAllBytes((Join-Path $here "icon-64.rgba"), $raw)
Write-Host "icon.ico, icon-256.png, icon-64.rgba written to $here"
