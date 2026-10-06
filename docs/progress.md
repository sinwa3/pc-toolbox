# 진행 상황

마지막 갱신: 2026-10-06

## 지금 위치 / 다음 할 일

**다음: 첫 유저스크립트 정하기** (미정 사항 참고)
- 유저스크립트 규칙(헤더, raw 링크, README 형식)은 `.claude/rules/userscripts.md`에 정리됨
- 만들기 전에 Greasy Fork 등에 이미 있는 스크립트부터 찾아 비교한다

## 마일스톤

| 단계 | 상태 | 비고 |
|---|---|---|
| 킥오프 (CLAUDE.md, rules, 훅, git) | 완료 2026-10-06 | |
| 화면 돋보기 도구 | 완료 2026-10-06 | 직접 만들지 않고 Windows 기본 돋보기 사용 |
| GitHub 연결 | 완료 2026-10-06 | github.com/sinwa3/pc-toolbox (public) |
| 유저스크립트 지원 | 진행 중 | 규칙·README 형식 완료, 첫 스크립트 미정 |

## 설계 결정

- `.sh` 훅은 `.gitattributes`로 LF 고정. 이 PC는 `core.autocrlf=true`라서 CRLF로 체크아웃되면 bash 훅이 실행되지 않는다.
- 돋보기: Windows 기본 돋보기 사용(확대/축소 간격 5%, Ctrl+Alt+휠). 사용자가 써 보고 충분하다고 판단. ZoomIt은 휠 한 칸에 2배씩 바뀌어 "연속 확대" 요구에 안 맞고, Virtual Magnifying Glass 등은 2019년 이후 유지보수 중단. 부족해지면 C# + Magnification API(소수 배율 지원)로 직접 만드는 안이 있다.
- 브라우저 유저스크립트(Tampermonkey)도 이 프로젝트의 `tools/<이름>/`에 둔다. 스크립트가 많아지면 별도 저장소 분리를 다시 검토한다.
- GitHub 저장소는 public. 유저스크립트를 raw 링크로 설치하고 `@updateURL`로 자동 업데이트하려면 공개여야 한다(비공개 raw 링크는 토큰이 필요해 Tampermonkey가 못 받는다). 그래서 개인정보·토큰은 커밋하지 않는다.

## 미정 사항

- 첫 유저스크립트로 무엇을 만들지

## 알려진 문제

- 없음
