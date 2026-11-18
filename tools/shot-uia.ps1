Add-Type -AssemblyName System.Drawing,UIAutomationClient,UIAutomationTypes
$p = Get-Process -Name freebuff-rune -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $p) { Write-Output "no-window"; exit 1 }
$el = [System.Windows.Automation.AutomationElement]::FromHandle($p.MainWindowHandle)
try { $el.SetFocus() } catch { }
Start-Sleep -Seconds 2
$r = $el.Current.BoundingRectangle
Write-Output ("rect {0},{1} {2}x{3} title={4}" -f [int]$r.X, [int]$r.Y, [int]$r.Width, [int]$r.Height, $el.Current.Name)
$bmp = New-Object System.Drawing.Bitmap ([int]$r.Width), ([int]$r.Height)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen([int]$r.X, [int]$r.Y, 0, 0, $bmp.Size)
$out = if ($args.Count -gt 0) { $args[0] } else { Join-Path (Split-Path $PSScriptRoot -Parent) "shots/window.png" }
New-Item -ItemType Directory -Force -Path (Split-Path $out -Parent) | Out-Null
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
Write-Output ("saved {0}" -f $out)
$g.Dispose(); $bmp.Dispose()
