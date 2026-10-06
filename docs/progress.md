# 진행 상황

마지막 갱신: 2026-10-06

## 지금 위치 / 다음 할 일

**soop-timeline v0.1.2 배포됨.** 사용자가 Firefox/Brave에서 써 보며 피드백 주는 단계
- 아직 확인 못 한 것: 전체화면에서 패널 표시, Firefox 동작 (알려진 문제 참고)
- 다음 도구는 미정. 만들기 전에 Greasy Fork 등에 이미 있는 것부터 찾아 비교한다

## 마일스톤

| 단계 | 상태 | 비고 |
|---|---|---|
| 킥오프 (CLAUDE.md, rules, 훅, git) | 완료 2026-10-06 | |
| 화면 돋보기 도구 | 완료 2026-10-06 | 직접 만들지 않고 Windows 기본 돋보기 사용 |
| GitHub 연결 | 완료 2026-10-06 | github.com/sinwa3/pc-toolbox (public) |
| 유저스크립트 지원 | 완료 2026-10-06 | 규칙·README 형식, 첫 스크립트 soop-timeline |

## 설계 결정

- `.sh` 훅은 `.gitattributes`로 LF 고정. 이 PC는 `core.autocrlf=true`라서 CRLF로 체크아웃되면 bash 훅이 실행되지 않는다.
- 돋보기: Windows 기본 돋보기 사용(확대/축소 간격 5%, Ctrl+Alt+휠). 사용자가 써 보고 충분하다고 판단. ZoomIt은 휠 한 칸에 2배씩 바뀌어 "연속 확대" 요구에 안 맞고, Virtual Magnifying Glass 등은 2019년 이후 유지보수 중단. 부족해지면 C# + Magnification API(소수 배율 지원)로 직접 만드는 안이 있다.
- 브라우저 유저스크립트(Tampermonkey)도 이 프로젝트의 `tools/<이름>/`에 둔다. 스크립트가 많아지면 별도 저장소 분리를 다시 검토한다.
- GitHub 저장소는 public. 유저스크립트를 raw 링크로 설치하고 `@updateURL`로 자동 업데이트하려면 공개여야 한다(비공개 raw 링크는 토큰이 필요해 Tampermonkey가 못 받는다). 그래서 개인정보·토큰은 커밋하지 않는다.
- soop-timeline: 기존 후보 중 왁타임라인(크롬 확장, 약 3,000명)은 Firefox 미지원, "SOOP 타임라인 활성기"(유저스크립트)는 화면에 불러온 댓글만 읽고 라이선스가 비공개라 고쳐 쓸 수 없어서 직접 만들었다.
- soop-timeline: 댓글은 화면의 "더보기"를 누르지 않고 숲 댓글 API로 직접 받는다(빠르고 화면 구조 변화에 덜 민감). 여러 사람의 타임라인은 작성자별로 묶고(타임스탬프 3개 이상), 1~2개짜리는 "한 줄 댓글", 그리고 "전체 합치기"를 따로 둔다. 기본 보기는 가장 많은 작성자.
- soop-timeline: 긴 다시보기는 여러 파일로 나뉘어 `video.currentTime`이 안 맞으므로, 숲의 `a.time_link[data-time]` 위임 클릭 처리를 빌려 시간 이동한다.

## 미정 사항

- 다음에 만들 도구

## 알려진 문제

- soop-timeline: 전체화면은 테스트 브라우저에서 동작하지 않아 미확인. 전체화면 요소로 패널을 옮기는 처리만 넣어 둠
- soop-timeline: Firefox에서는 아직 미확인 (Chromium에서만 시험)
