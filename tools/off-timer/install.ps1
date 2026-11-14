# 끄기 타이머 설치: exe를 %LOCALAPPDATA%\Programs\off-timer\에 복사하고 시작 메뉴 바로가기를 만든다.
$ErrorActionPreference = "Stop"
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$exe = Join-Path $here "target\release\off-timer.exe"

if (-not (Test-Path $exe)) {
    Write-Host "빌드된 exe가 없어서 cargo build --release를 실행합니다..."
    $cargo = Get-Command cargo -ErrorAction SilentlyContinue
    if (-not $cargo) {
        $cargoPath = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
        if (Test-Path $cargoPath) { $cargo = $cargoPath } else { throw "cargo를 찾을 수 없어요. Rust(https://rustup.rs/)를 먼저 설치해 주세요." }
    }
    Push-Location $here
    try { & $cargo build --release; if ($LASTEXITCODE -ne 0) { throw "빌드에 실패했어요." } } finally { Pop-Location }
}

$dest = Join-Path $env:LOCALAPPDATA "Programs\off-timer"
New-Item -ItemType Directory -Force $dest | Out-Null
Get-Process off-timer -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq (Join-Path $dest "off-timer.exe") } | ForEach-Object {
    throw "끄기 타이머가 실행 중이에요. 창을 닫고 다시 실행해 주세요."
}
Copy-Item $exe (Join-Path $dest "off-timer.exe") -Force

$lnk = Join-Path ([Environment]::GetFolderPath("Programs")) "끄기 타이머.lnk"
$shell = New-Object -ComObject WScript.Shell
$s = $shell.CreateShortcut($lnk)
$s.TargetPath = Join-Path $dest "off-timer.exe"
$s.WorkingDirectory = $dest
$s.Description = "프로그램과 PC를 정한 시간 뒤에 끄는 타이머"
$s.Save()

Write-Host "설치했어요."
Write-Host "  실행 파일: $dest\off-timer.exe"
Write-Host "  시작 메뉴: 끄기 타이머"
