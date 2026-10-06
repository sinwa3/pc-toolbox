#!/bin/bash
# PreToolUse(Edit|Write) 훅: 프로젝트 폴더 밖 파일 수정을 차단한다. exit 2 = 차단, stderr는 Claude에게 전달된다.
# jq 없이 동작하도록 grep/sed로 file_path를 뽑는다.

INPUT=$(cat)
FILE_PATH=$(printf '%s' "$INPUT" \
  | grep -o '"file_path"[[:space:]]*:[[:space:]]*"[^"]*"' \
  | head -n 1 \
  | sed 's/^"file_path"[[:space:]]*:[[:space:]]*"//; s/"$//')

[ -z "$FILE_PATH" ] && exit 0

# 경로 정규화: 역슬래시 → /, /d/... → d:/..., 소문자화(Windows는 대소문자 무시), 끝 / 보장
norm() {
  local p
  p=$(printf '%s' "$1" | sed 's#\\\\#/#g; s#\\#/#g; s#//*#/#g')
  if [[ "$p" =~ ^/([a-zA-Z])/(.*)$ ]]; then
    p="${BASH_REMATCH[1]}:/${BASH_REMATCH[2]}"
  fi
  printf '%s' "${p,,}"
}

TARGET=$(norm "$FILE_PATH")

# 상위 폴더로 빠져나가는 경로는 판단하지 않고 차단
if [[ "$TARGET" == *"/../"* || "$TARGET" == *"/.." ]]; then
  echo "차단됨: '$FILE_PATH' 는 '..'이 들어간 경로다. 정규화된 절대 경로로 다시 시도할 것." >&2
  exit 2
fi

# 허용 위치: 프로젝트 폴더, Claude 설정·메모리(~/.claude), Claude 임시 폴더
ALLOWED=(
  "$(norm "$CLAUDE_PROJECT_DIR")/"
  "$(norm "$HOME")/.claude/"
  "$(norm "$HOME")/appdata/local/temp/claude/"
)

for prefix in "${ALLOWED[@]}"; do
  [[ "$prefix" == "/" ]] && continue
  if [[ "$TARGET" == "$prefix"* ]]; then
    exit 0
  fi
done

echo "차단됨: '$FILE_PATH' 는 프로젝트 폴더 밖이다. 폴더 밖 파일은 Claude가 수정하지 않는다 — 사용자가 직접 하도록 방법을 안내할 것." >&2
exit 2
