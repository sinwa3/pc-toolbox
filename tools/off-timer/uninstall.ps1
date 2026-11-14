# 끄기 타이머 제거: install.ps1이 만든 exe 폴더와 시작 메뉴 바로가기를 지운다.
$ErrorActionPreference = "Stop"
$dest = Join-Path $env:LOCALAPPDATA "Programs\off-timer"
$lnk = Join-Path ([Environment]::GetFolderPath("Programs")) "끄기 타이머.lnk"

Get-Process off-timer -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq (Join-Path $dest "off-timer.exe") } | ForEach-Object {
    throw "끄기 타이머가 실행 중이에요. 창을 닫고 다시 실행해 주세요."
}

if (Test-Path $lnk) { Remove-Item $lnk -Force; Write-Host "시작 메뉴 바로가기를 지웠어요." }
if (Test-Path $dest) { Remove-Item $dest -Recurse -Force; Write-Host "$dest 를 지웠어요." }
Write-Host "제거했어요. (저장소 안의 소스와 빌드 결과는 그대로 둡니다)"
