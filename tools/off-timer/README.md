# 끄기 타이머 (off-timer)

**용도**: 고른 프로그램을 정한 시간 뒤에 끄고, PC도 정한 시간 뒤에 종료·다시 시작·절전·최대 절전한다. 프로그램 타이머는 여러 개를 동시에 걸 수 있고, PC 타이머와 따로 돈다.

Rust + egui(eframe)로 만든 이유: 실행 파일 하나(약 5MB)로 끝나서 Python 없는 PC에서도 바로 쓸 수 있고, 켜지는 속도가 빠르고 메모리를 적게 쓴다(사용자 선택). 기존 무료 프로그램 중 Wise Auto Shutdown은 같은 회사 제품 광고가 있고, AutoClose 무료판은 작업 2개·30분 제한이 있어 직접 만들었다.

## 사용법

### 프로그램 끄기
1. 목록에서 끌 프로그램을 고른다. 창이 떠 있는 프로그램만 나오고, 실행 파일 이름(예: `chrome.exe`)으로 묶인다. 3초마다 자동으로 새로 읽고, 검색칸으로 좁힐 수 있다.
2. 시간을 정한다. **30분 / 1시간 / 2시간** 버튼을 누르거나 입력칸에 직접 넣는다.
   - 숫자만 넣으면 분: `90` → 90분 뒤
   - `:`를 넣으면 시각: `02:30` → 오전 2시 30분(이미 지났으면 내일)
   - 아래에 "→ 오후 11:42에 꺼져요"처럼 언제인지 미리 보인다.
3. **추가**를 누르면 아래 목록에 남은 시간이 보인다. 줄마다 **취소**가 있다.

시간이 되면:
1. 그 프로그램의 모든 창에 정상 종료를 요청한다(창의 X를 누른 것과 같음).
2. 30초 안에 꺼지지 않으면 강제 종료한다. 추가할 때 "30초 안에 안 꺼지면 강제 종료"를 끄면 강제 종료하지 않고 "안 꺼졌어요"로 남긴다. 이때 줄의 **강제로 끄기**로 직접 끌 수 있다.
3. 실행 파일 이름이 같은 프로세스는 모두 끈다(창 없는 보조 프로세스 포함).

### PC 끄기
1. **종료 / 다시 시작 / 절전 / 최대 절전** 중 하나를 고른다.
2. 시간을 정하고 **시작**을 누른다. 남은 시간이 크게 보이고 **취소**로 멈출 수 있다.
3. 끄기 60초 전에 창이 (내려 둔 상태여도) 다시 떠서 맨 위에 고정되고 경고 상자가 나온다. **취소** 또는 **10분 미루기**를 누를 수 있다.
4. "저장 안 된 프로그램이 있어도 끄기"(종료·다시 시작만)를 켜면 `shutdown /f`로 끈다. 저장 안 한 작업을 잃을 수 있어서 기본은 꺼져 있다. 끄지 않으면 저장할지 묻는 프로그램이 있을 때 Windows가 종료를 멈추고 기다린다.

### 그 밖의 동작
- 타이머가 하나라도 돌고 있으면 PC가 저절로 절전에 들어가지 않는다. 이 프로그램이 켜져 있는 동안만 적용되고 전원 설정은 바꾸지 않는다.
- 타이머가 남아 있는데 창을 닫으면 "모두 취소하고 닫기"를 한 번 더 묻는다. 창을 닫으면 타이머도 모두 사라진다. 창은 닫지 말고 내려(최소화) 둔다.
- 설정·기록은 저장하지 않는다.
- 다크/라이트는 Windows 설정을 따른다.

## 실행 방법

### 받아서 실행
[Releases](https://github.com/sinwa3/pc-toolbox/releases)에서 `off-timer.exe`를 받아 더블클릭한다. 설치가 필요 없고 exe 하나로 실행된다(Windows 10/11 64비트). Visual C++ 런타임도 exe 안에 들어 있다. 인터넷에서 받은 서명 없는 exe라 처음 실행할 때 SmartScreen이 "Windows의 PC 보호" 창을 띄우면 "추가 정보" → "실행"을 누른다.

### 직접 빌드
[Rust](https://rustup.rs/)와 Visual Studio C++ 빌드 도구가 필요하다.

```powershell
cd tools\off-timer
cargo build --release
```

`target\release\off-timer.exe`를 더블클릭하면 된다. 이 exe 하나만 다른 곳에 복사해 써도 된다.

## 설치·제거

설치하면 exe를 `%LOCALAPPDATA%\Programs\off-timer\`에 복사하고 시작 메뉴에 "끄기 타이머" 바로가기를 만든다. 빌드된 exe가 없으면 먼저 `cargo build --release`를 실행한다. 관리자 권한은 필요 없다.

- 설치: `install.bat` 더블클릭
- 제거: `uninstall.bat` 더블클릭 (복사한 exe 폴더와 시작 메뉴 바로가기를 지운다)

업데이트는 소스를 받은 뒤 `cargo build --release` → `install.bat`을 다시 실행한다(실행 중이면 먼저 창을 닫는다).

## 시험용

환경 변수 `OFF_TIMER_DRY_RUN=1`을 주고 실행하면 PC 동작을 실제로 하지 않고 "시험 모드" 문구만 보인다. 프로그램 끄기는 그대로 동작한다.

```powershell
$env:OFF_TIMER_DRY_RUN = "1"; .\target\release\off-timer.exe
```

## 배포 (새 버전 올리기)

1. `Cargo.toml`의 `version`을 올린다.
2. `powershell -NoProfile -ExecutionPolicy Bypass -File release.ps1 -Version <버전> [-NotesFile 설명.md]`
   - release 빌드 → `cargo about generate`로 `THIRD-PARTY-LICENSES.txt`(라이브러리·글꼴 저작권 표시와 라이선스 전문) 생성 → GitHub 릴리즈 `off-timer-v<버전>`에 exe와 함께 올린다. 같은 태그가 있으면 파일만 덮어쓴다.
   - `-NoticesOnly`로 고지문만 만들어 볼 수 있다.
   - 필요한 것: `cargo install cargo-about --locked --features cli`, `gh` 로그인.
3. `about.toml`의 `accepted`에 없는 라이선스(GPL 등)가 의존성에 들어오면 고지문 생성이 실패한다. 목록에 넣기 전에 그 라이브러리를 써도 되는지 먼저 정한다.

## 라이선스

MIT (저장소 루트 `LICENSE`). 함께 들어간 오픈소스 라이브러리·글꼴의 라이선스는 릴리즈의 `THIRD-PARTY-LICENSES.txt`.

## 알려진 제한

- 관리자 권한으로 실행된 프로그램은 끌 수 없다("끄지 못했어요"로 표시).
- 탐색기(`explorer.exe`)처럼 끄면 Windows 화면이 망가지는 프로그램은 목록에서 빠진다. 설정·계산기 같은 일부 스토어 앱은 `ApplicationFrameHost.exe` 아래에 있어 목록에 안 나올 수 있다.
- 닫기 버튼을 누르면 트레이로 숨는 프로그램(Discord, Steam 등)은 정상 종료 요청으로는 안 꺼지고 30초 뒤 강제 종료된다.
- PC가 꺼져 있거나 절전 중이었다가 깨어났을 때 시간이 이미 지났으면, 바로 실행하지 않고 경고 60초를 새로 준다.

## 동작 방식 (고칠 때 참고)

- `src/win.rs`: Windows API. 창 목록(`EnumWindows` → 보이고, 주인 창 없고, 도구 창·숨겨진 UWP 창이 아닌 것), 정상 종료(`WM_CLOSE`), 프로세스 찾기(`CreateToolhelp32Snapshot`), 강제 종료(`TerminateProcess`), 절전 막기(`SetThreadExecutionState`), PC 동작(`shutdown.exe /s|/r|/h /t 0`, 절전은 `SetSuspendState`).
- `src/main.rs`: 화면과 타이머. 타이머 판정은 `App::logic`에서 해서 창을 내려 둬도 돈다. 마감 시각은 `SystemTime`(벽시계)으로 저장한다.
- 경고는 별도 창이 아니라 메인 창 안의 상자다. egui 0.37에서 내려 둔 창이 별도 창(immediate viewport)을 만들면 패닉이 나서, 경고 시간에 메인 창을 다시 띄워 맨 위로 고정한다.
- 한글은 egui 기본 글꼴에 없어서 실행할 때 맑은 고딕(`C:\Windows\Fonts\malgun.ttf`, `malgunbd.ttf`)을 불러온다.
- 아이콘: 전원 기호의 세로선을 12시 분침으로 두고 4시 방향 시침을 더한 모양. `assets/make-icon.ps1`이 System.Drawing으로 그려 `icon.ico`(16~128px는 BMP, 256px는 PNG — 오래된 API가 작은 PNG 항목을 못 읽음), `icon-256.png`(미리보기), `icon-64.rgba`(창 아이콘)를 만든다. 고치려면 스크립트 값을 바꾸고 `powershell -NoProfile -ExecutionPolicy Bypass -File assets\make-icon.ps1` 실행 후 다시 빌드한다.
- `.cargo/config.toml`에서 C 런타임을 정적 링크(`+crt-static`)해서 Visual C++ 재배포 패키지 없이 실행된다. exe가 쓰는 DLL은 Windows 기본 DLL(`kernel32`, `user32`, `opengl32` 등)뿐이다.
- exe 아이콘은 `build.rs`가 `winresource`로 넣는다(Windows SDK의 `rc.exe` 필요, VS C++ 빌드 도구에 포함). 창·작업 표시줄 아이콘은 `main.rs`에서 `icon-64.rgba`를 `with_icon`으로 넣는다.
