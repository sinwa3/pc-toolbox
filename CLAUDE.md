# 컴퓨터용 프로그램

개인 PC(Windows 11)용 편의 도구와 바로가기를 모아두는 툴박스. 필요할 때마다 작은 도구를 하나씩 추가한다.

## 환경
- Windows 11, 기본 셸은 Windows PowerShell 5.1
- 설치된 것: Python 3.14, .NET 9, Node 24, git

## 구조
- 도구 하나 = `tools/<도구이름>/` 폴더 하나. 폴더 안에 `README.md`(용도, 실행 방법, 설치·제거 방법)를 둔다.
- 새 도구를 만들면 루트 `README.md` 목록에 한 줄 추가.
- 공유하지 않을 도구는 `private/tools/<이름>/`에 둔다. `private/`는 공개 저장소에서 `.gitignore`로 빠지고, 안에 별도 비공개 저장소(`sinwa3/pc-toolbox-private`)가 있다. 목록은 `private/README.md`에 적고 루트 README에는 적지 않는다. 새 도구를 만들 때 공개/비공개를 먼저 묻는다.

## 언어 고르는 기준
- 바로가기, 파일 정리, 간단한 자동화 → PowerShell(.ps1) 또는 배치(.bat)
- 창이 필요한 프로그램 → Python(tkinter 등) 또는 C#(WinForms/WPF). 도구마다 이유를 README에 한 줄 남긴다.
- 브라우저 동작 바꾸기 → 유저스크립트(`.user.js`, Tampermonkey). 시스템을 바꾸지 않으므로 `install.ps1`은 만들지 않는다.
- 외부 라이브러리는 꼭 필요할 때만 쓴다. Python이면 도구 폴더에 `requirements.txt`를 둔다.

## 작업 방식
- 새 도구를 만들기 전에 이미 널리 쓰이는 무료 프로그램이 있는지 먼저 찾아본다(PowerToys, AutoHotkey 스크립트, Everything 등). 있으면 "직접 만들기"와 비교해서 먼저 제안한다. 출처, 사용자 규모, 유지보수 상태도 함께 알려준다.
- 시작프로그램 등록, 단축키 등록, 작업 스케줄러, 레지스트리, 환경변수처럼 시스템을 바꾸는 작업은 Claude가 직접 실행하지 않는다. 사용자가 실행할 `install.ps1`과 `uninstall.ps1`을 도구 폴더에 만들어 준다.

## 문서 지도
- 지금 위치, 다음 할 일, 마일스톤, 설계 결정, 미정 사항, 알려진 문제 → `docs/progress.md`
- 도구별 사용법 → `tools/<이름>/README.md`, 도구 목록 → 루트 `README.md`

@docs/progress.md

## 하지 말 것
- 이 폴더 밖의 파일은 셸 명령(cp, mv, Remove-Item 등)으로도 수정하거나 삭제하지 않는다. 읽기는 괜찮다. (Edit/Write는 훅이 막지만 셸 명령은 못 막는다)
- 저장소가 public이다. 개인정보, 토큰, 개인 경로가 담긴 설정은 커밋하지 않는다(`private/` 안은 비공개 저장소라 예외지만 토큰은 거기서도 커밋하지 않는다).

<!-- 이 파일은 60줄 이하를 목표로 한다. 같은 실수가 두 번 나올 때만 한 줄씩 추가.
     절차 → .claude/skills/, 폴더 한정 규칙 → .claude/rules/ (paths 필수), 절대 금지 → hooks -->
<!-- kickoff: profile=script, purpose=personal-use, date=2026-10-06, deferred=claude-code-setup(도구 5개쯤),mcp-builder(외부 서비스 연결 시),project-skill(같은 종류 도구 반복 시), installed= -->
