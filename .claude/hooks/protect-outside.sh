#!/bin/bash
# PreToolUse(Edit|Write) 훅: 프로젝트 폴더 밖 파일 수정을 차단한다. exit 2 = 차단, stderr는 Claude에게 전달된다.
# Windows는 프로세스 생성이 느려서 외부 명령(grep/sed/cat)과 $(...) 없이 bash 내장 기능만 쓴다.

IFS= read -r -d '' INPUT
re='"file_path"[[:space:]]*:[[:space:]]*"([^"]*)"'
[[ "$INPUT" =~ $re ]] || exit 0
FILE_PATH="${BASH_REMATCH[1]}"

# 경로 정규화 → REPLY: 역슬래시 → /, 연속 / 하나로, /d/... → d:/..., 소문자화(Windows는 대소문자 무시)
norm() {
  local p="${1//\\//}"
  while [[ "$p" == *//* ]]; do p="${p//\/\//\/}"; done
  if [[ "$p" =~ ^/([a-zA-Z])/(.*)$ ]]; then
    p="${BASH_REMATCH[1]}:/${BASH_REMATCH[2]}"
  fi
  REPLY="${p,,}"
}

norm "$FILE_PATH"; TARGET="$REPLY"

# 상위 폴더로 빠져나가는 경로는 판단하지 않고 차단
if [[ "$TARGET" == *"/../"* || "$TARGET" == *"/.." ]]; then
  echo "차단됨: '$FILE_PATH' 는 '..'이 들어간 경로다. 정규화된 절대 경로로 다시 시도할 것." >&2
  exit 2
fi

# 허용 위치: 프로젝트 폴더, Claude 설정·메모리(~/.claude), Claude 임시 폴더
norm "$CLAUDE_PROJECT_DIR"; PROJECT="$REPLY"
norm "$HOME"; HOMEDIR="$REPLY"
ALLOWED=(
  "$PROJECT/"
  "$HOMEDIR/.claude/"
  "$HOMEDIR/appdata/local/temp/claude/"
)

for prefix in "${ALLOWED[@]}"; do
  [[ "$prefix" == "/" ]] && continue
  [[ "$TARGET" == "$prefix"* ]] && exit 0
done

echo "차단됨: '$FILE_PATH' 는 프로젝트 폴더 밖이다. 폴더 밖 파일은 Claude가 수정하지 않는다 — 사용자가 직접 하도록 방법을 안내할 것." >&2
exit 2
