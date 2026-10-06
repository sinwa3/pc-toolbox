#!/bin/bash
# PreToolUse(Bash|PowerShell) 훅: 시스템 설정을 바꾸는 명령을 실행 전에 차단한다. exit 2 = 차단.
# tool_input 부분만 잘라 대소문자 무시로 패턴 검사한다. 읽기(조회) 명령은 막지 않는다.
# Windows는 프로세스 생성이 느려서 외부 명령(grep/sed/cat) 없이 bash 내장 정규식만 쓴다.

IFS= read -r -d '' INPUT
CMD="${INPUT#*\"tool_input\"}"
shopt -s nocasematch

# 패턴: ERE. 새로 막고 싶은 명령은 여기에 한 줄 추가.
PATTERNS=(
  '\breg(\.exe)?[[:space:]]+(add|delete|import|copy|restore|load|unload)\b'   # 레지스트리 (reg query는 허용)
  '\bregedit\b'
  '\b(new|set|remove|rename|copy|move|clear)-itemproperty\b'                  # 레지스트리 값 쓰기
  '\b(new|set|remove|clear)-item\b.*(hklm|hkcu|hkey_|registry::)'             # 레지스트리 키 쓰기
  '\bsetx\b'                                                                  # 환경변수 영구 설정
  'setenvironmentvariable'
  '\bschtasks(\.exe)?[[:space:]]+/(create|delete|change|run)'                 # 작업 스케줄러
  '\b(register|unregister|set|enable|disable|start)-scheduledtask\b'
  '\bset-executionpolicy\b'
  '\bsc(\.exe)?[[:space:]]+(create|config|delete|stop|start)\b'               # 서비스
  '\b(new|set|remove|stop|start|restart)-service\b'
  '\bbcdedit\b'
  '\bnetsh\b.*\b(add|set|delete|reset)\b'
  '\b(new|set|remove|enable|disable)-netfirewallrule\b'
  '\b(set|add|remove)-mppreference\b'                                         # Defender
  'programs[\\/]+startup|shell:startup|shell:common startup'                  # 시작프로그램 폴더
  '(-file[[:space:]]+|&[[:space:]]*|/c[[:space:]]+|start-process[[:space:]]+).*\b(un)?install\.(ps1|bat|cmd)'  # install/uninstall 스크립트 실행
)

for pattern in "${PATTERNS[@]}"; do
  if [[ "$CMD" =~ $pattern ]]; then
    echo "차단됨: 시스템 설정을 바꾸는 명령이다(패턴: $pattern). Claude가 실행하지 말고, 사용자가 직접 실행할 명령이나 도구 폴더의 install.ps1로 안내할 것." >&2
    exit 2
  fi
done

exit 0
