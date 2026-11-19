# 끄기 타이머 배포: release 빌드 → 라이브러리 고지문 생성 → GitHub 릴리즈에 올리기.
# 사용: powershell -NoProfile -ExecutionPolicy Bypass -File release.ps1 -Version 0.1.1 [-NotesFile notes.md] [-NoticesOnly]
#   -Version     Cargo.toml의 version과 같아야 한다. 태그는 off-timer-v<버전>.
#   -NotesFile   새 릴리즈를 만들 때 쓸 설명(Markdown). 없으면 기본 문구.
#   -NoticesOnly 빌드·업로드 없이 고지문만 만든다(확인용).
# 같은 태그의 릴리즈가 이미 있으면 파일만 덮어써 올린다.
param(
    [Parameter(Mandatory = $true)][string]$Version,
    [string]$NotesFile = "",
    [switch]$NoticesOnly
)
$ErrorActionPreference = "Stop"
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$repo = "sinwa3/pc-toolbox"
$tag = "off-timer-v$Version"
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"

foreach ($cmd in "cargo", "cargo-about", "gh") {
    if (-not (Get-Command $cmd -ErrorAction SilentlyContinue)) { throw "$cmd 을(를) 찾을 수 없어요." }
}

$toml = Get-Content (Join-Path $here "Cargo.toml") -Raw
if ($toml -notmatch '(?m)^version\s*=\s*"([^"]+)"') { throw "Cargo.toml에서 version을 못 찾았어요." }
if ($Matches[1] -ne $Version) { throw "Cargo.toml version($($Matches[1]))과 -Version($Version)이 달라요. 먼저 맞춰 주세요." }

Push-Location $here
try {
    $out = Join-Path $here "target\release"
    $notices = Join-Path $out "THIRD-PARTY-LICENSES.txt"
    $exe = Join-Path $out "off-timer.exe"

    if (-not $NoticesOnly) {
        Write-Host "1/3 release 빌드..."
        cargo build --release
        if ($LASTEXITCODE -ne 0) { throw "빌드에 실패했어요." }
    }

    Write-Host "2/3 라이브러리 고지문 생성..."
    New-Item -ItemType Directory -Force $out | Out-Null
    cargo about generate about.hbs -o $notices
    if ($LASTEXITCODE -ne 0) { throw "고지문 생성에 실패했어요. about.toml의 accepted에 없는 라이선스가 있는지 확인해 주세요." }
    Write-Host ("   {0} ({1:N0} KB)" -f $notices, ((Get-Item $notices).Length / 1KB))
    if ($NoticesOnly) { return }

    Write-Host "3/3 GitHub 릴리즈 $tag ..."
    gh release view $tag --repo $repo *> $null
    if ($LASTEXITCODE -eq 0) {
        gh release upload $tag $exe $notices --repo $repo --clobber
    } else {
        $notesArgs = if ($NotesFile) { @("--notes-file", $NotesFile) } else { @("--notes", "끄기 타이머 v$Version. 사용법: https://github.com/$repo/blob/main/tools/off-timer/README.md") }
        gh release create $tag $exe $notices --repo $repo --target main --title "끄기 타이머 v$Version" @notesArgs
    }
    if ($LASTEXITCODE -ne 0) { throw "릴리즈 올리기에 실패했어요." }
    Write-Host "완료: https://github.com/$repo/releases/tag/$tag"
} finally {
    Pop-Location
}
