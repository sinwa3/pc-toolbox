---
paths:
  - "**/*.user.js"
---

# 유저스크립트 규칙

- 파일 이름은 `tools/<이름>/<이름>.user.js`. 헤더에 `@name`, `@namespace`, `@version`, `@description`, `@match`, `@grant`, `@updateURL`, `@downloadURL`을 넣는다. `@grant`가 필요 없으면 `@grant none`.
- raw 링크: `https://raw.githubusercontent.com/sinwa3/pc-toolbox/main/tools/<이름>/<이름>.user.js` (`@updateURL`, `@downloadURL`도 이 주소)
- `@match`는 실제로 쓰는 사이트로 좁힌다. `*://*/*`는 꼭 필요할 때만.
- 수정할 때마다 `@version`을 올린다. 버전이 올라가야 Tampermonkey가 업데이트로 인식한다. push해야 자동 업데이트에 반영된다.
- 도구 폴더 README는 아래 순서로 쓴다:
  - **용도**: 한두 줄
  - **대상 사이트**: `@match` 목록
  - **설치**: raw 링크를 브라우저에서 열고 Tampermonkey 설치 화면에서 "설치". push 전이라면 대시보드 → 새 스크립트 → `.user.js` 내용 전체 붙여넣기.
  - **제거**: Tampermonkey 대시보드에서 해당 스크립트 삭제
  - **업데이트**: 자동(`@updateURL`). 바로 받으려면 대시보드에서 "업데이트 확인"
