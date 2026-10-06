---
paths:
  - "**/*.ps1"
  - "**/*.bat"
  - "**/*.cmd"
---

# Windows 스크립트 규칙

- .ps1에 한글이 들어가면 **UTF-8 with BOM**으로 저장한다. Windows PowerShell 5.1은 BOM 없는 파일을 ANSI로 읽어 한글이 깨진다. Write 도구는 BOM을 넣지 않으므로, 저장한 뒤 BOM을 붙이거나 처음부터 한글 없이 쓴다.
- .bat/.cmd에 한글 출력이 있으면 첫 줄 근처에 `chcp 65001 > nul`을 넣고 UTF-8(BOM 없음)으로 저장한다.
- PowerShell 5.1 기준으로 작성한다: `&&`, `||`, `?:`, `??`는 쓸 수 없다.
- 사용자가 더블클릭으로 실행할 .ps1은 실행 정책에 막히지 않게 같은 이름의 .bat 런처를 함께 준다: `powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0이름.ps1"`
