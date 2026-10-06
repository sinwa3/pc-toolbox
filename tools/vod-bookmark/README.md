# 다시보기 책갈피 (숲·치지직)

**용도**: 숲·치지직 다시보기에서 지금 시간을 메모와 함께 책갈피로 남긴다(영상당 5개). 어느 화면에서든 책갈피한 영상 목록을 열어 그 시간으로 바로 간다.

유저스크립트로 만든 이유: 브라우저 동작만 바꾸면 되고, Firefox와 Brave에서 같이 쓰기 위해서. 스크립트 하나로 두 사이트를 지원해야 책갈피가 한 목록(Tampermonkey 저장소는 스크립트별)에 모인다. 기존 "치지직 나중에 보기"는 영상 단위 저장만 되고 시간 책갈피가 없다.

## 대상 사이트

- `https://chzzk.naver.com/*`
- `https://*.sooplive.com/*`
- `https://*.sooplive.co.kr/*`

책갈피 추가는 다시보기 화면(숲 `vod.sooplive.*/player/<번호>`, 치지직 `chzzk.naver.com/video/<번호>`)에서만 된다. 목록 버튼은 두 사이트 모든 화면에 뜬다.

## 사용법

- **추가**: 다시보기 화면에서 재생바 오른쪽의 책갈피 아이콘을 누르거나 `B` 키 → 왼쪽 위 패널. 메모를 쓰고 Enter(또는 "+ 지금")를 누르면 지금 시간이 저장된다. 메모는 비워도 된다.
- 패널의 책갈피를 누르면 그 시간으로 이동, `✕`는 삭제. 영상당 5개가 차면 추가 버튼이 꺼진다(오래된 것을 자동으로 지우지 않는다).
- **목록**: 화면 왼쪽 아래 둥근 책갈피 버튼 → 책갈피한 영상 목록(최근에 추가한 순, 숲/치지직 표시).
  - 제목을 누르면 영상을 연다(시간 지정 없음, 사이트의 이어보기 위치가 있으면 그리로).
  - `▸`로 펼친 책갈피를 누르면 그 시간으로 연다. 지금 보는 영상이면 페이지를 다시 열지 않고 바로 이동한다.
  - 영상의 마지막 책갈피를 지우면 영상도 목록에서 빠진다.

## 설치

1. Tampermonkey가 설치된 브라우저에서 **[여기를 눌러 설치](https://raw.githubusercontent.com/sinwa3/pc-toolbox/main/tools/vod-bookmark/vod-bookmark.user.js)** (raw 링크가 열린다)
2. Tampermonkey 설치 화면에서 "설치"를 누른다.

push 전이라면 Tampermonkey 대시보드 → 새 스크립트 → `vod-bookmark.user.js` 내용 전체를 붙여넣고 저장한다.

## 제거

Tampermonkey 대시보드에서 "다시보기 책갈피 (숲·치지직)"을 삭제한다. 책갈피 데이터도 함께 지워진다.

## 업데이트

자동(`@updateURL`). 바로 받으려면 대시보드에서 "업데이트 확인"을 누른다.

## 동작 방식 (고칠 때 참고)

- 저장: `GM_setValue('videos', { "<사이트>:<번호>": { site, id, url, title, channel, updatedAt, marks: [{ sec, memo }] } })`. 브라우저(프로필)마다 따로 저장된다. 다른 탭에서 바꾸면 `GM_addValueChangeListener`로 바로 반영.
- 다른 영상의 특정 시간으로 갈 때는 주소에 `#bm=<초>`를 붙여 열고, 영상 페이지가 플레이어 준비를 기다렸다가(최대 30초) 이동한 뒤 주소에서 지운다. 플레이어가 이어보기 위치로 되돌리는 경우를 대비해 1.5초 뒤 한 번 더 확인한다.
- 치지직은 다른 영상으로 갈 때, 숲은 다음 영상 자동 재생 때 페이지를 새로 열지 않으므로 0.5초마다 주소가 바뀌었는지 본다.
- 숲
  - 영상 정보: `api.m.sooplive.co.kr/station/video/a/view` (`nTitleNo`). 실패하면 문서 제목을 쓴다.
  - 현재 시간: 재생바 `.time-current` 글자. 시간 이동: 숲의 `a.time_link[data-time]` 클릭 처리 빌려 쓰기(긴 다시보기는 여러 파일로 나뉘어 `video.currentTime`이 안 맞음, soop-timeline과 같은 방식).
  - 버튼 자리 `.right_ctrl`, 패널 자리 `#player`.
- 치지직
  - 영상 정보: `api.chzzk.naver.com/service/v2/videos/<번호>` → `videoTitle`, `channel.channelName`.
  - 본 영상 `video.webplayer-internal-video`(파일 하나라 `currentTime`이 전체 시간과 같음). 나머지 `<video>`는 광고용.
  - 버튼 자리 `.pzp-pc__bottom-buttons-right`, 패널 자리 `.pzp-pc`.
- 사이트가 이 주소, 클래스 이름, 응답 형식을 바꾸면 깨질 수 있다.
