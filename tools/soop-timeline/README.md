# SOOP 다시보기 댓글 타임라인

**용도**: 숲 다시보기의 댓글과 답글을 전부 불러와 타임라인(시간 + 소제목)만 모아 플레이어 오른쪽 반투명 패널에 보여준다. 항목을 누르면 그 시간으로 이동한다.

유저스크립트로 만든 이유: 브라우저 동작만 바꾸면 되고, Firefox와 Brave에서 같이 쓰기 위해서. 비슷한 "SOOP 타임라인 활성기"는 화면에 불러온 댓글만 읽고, 라이선스가 비공개라 고쳐 쓸 수 없어서 새로 만들었다.

## 대상 사이트

- `https://vod.sooplive.com/player/*`
- `https://vod.sooplive.co.kr/player/*`

## 사용법

- 재생바 오른쪽 버튼 묶음 맨 앞의 목록 아이콘을 누르거나 `T` 키를 누르면 패널이 열리고 닫힌다.
- 처음 열 때 댓글을 전부 불러온다. 댓글이 많으면 몇 초 걸린다. `↻`를 누르면 다시 불러온다.
- 위쪽 선택 메뉴에서 보기를 고른다.
  - **작성자별 타임라인**: 한 사람이 쓴 타임스탬프가 3개 이상이면 묶어서 보여준다. 여러 댓글이나 답글로 나눠 쓴 것도 합친다. 기본값은 가장 많은 사람이다.
  - **한 줄 댓글**: 타임스탬프가 1~2개뿐인 댓글을 모은다.
  - **전체 합치기**: 모두 시간순으로 합치고 작성자를 함께 표시한다.
- 검색창에 글자를 넣으면 맞는 항목만 남는다.
- 지금 재생 중인 구간이 초록색으로 강조되고 목록이 따라 내려간다. 목록을 직접 굴리면 3초 동안은 따라가지 않는다.

## 설치

1. Tampermonkey가 설치된 브라우저에서 아래 raw 링크를 연다.
   `https://raw.githubusercontent.com/sinwa3/pc-toolbox/main/tools/soop-timeline/soop-timeline.user.js`
2. Tampermonkey 설치 화면에서 "설치"를 누른다.

push 전이라면 Tampermonkey 대시보드 → 새 스크립트 → `soop-timeline.user.js` 내용 전체를 붙여넣고 저장한다.

## 제거

Tampermonkey 대시보드에서 "SOOP 다시보기 댓글 타임라인"을 삭제한다.

## 업데이트

자동(`@updateURL`). 바로 받으려면 대시보드에서 "업데이트 확인"을 누른다.

## 동작 방식 (고칠 때 참고)

- 영상 정보: `api.m.sooplive.com/station/video/a/view` (`nTitleNo`) → 방송국 번호, 게시판 번호, 영상 길이
- 댓글: `stbbs.sooplive.com/api/bbs_memo_action.php`
  - `szAction=get`으로 20개씩 받는다. 다음 페이지는 `nPageNo`를 올리고 `nLastNo`에 직전 페이지 마지막 댓글 번호를 넣는다. `has_more`가 false면 끝이다.
  - `szAction=get_reply&nParentCommentNo=<댓글 번호>`로 답글을 받는다.
- 시간 이동: 긴 다시보기는 여러 파일로 나뉘어 있어 `video.currentTime`으로는 못 옮긴다. 숲이 댓글 속 시간 링크(`a.time_link[data-time="초"]`)에 걸어 둔 클릭 처리를 빌려 쓴다.
- 현재 위치: 재생바의 `.time-current` 글자를 읽는다.
- 숲이 이 주소, 클래스 이름, 응답 형식을 바꾸면 깨질 수 있다.
