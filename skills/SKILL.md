---
name: ima-skill
description: |
  统一的 IMA OpenAPI 技能，支持笔记管理和知识库操作。
  当用户提到知识库、资料库、笔记、备忘录、记事，或者想要上传文件、添加网页到知识库、
  搜索知识库内容、搜索/浏览/创建/编辑笔记时，使用此 skill。
  即使用户没有明确说"知识库"或"笔记"，只要意图涉及文件上传到知识库、网页收藏、
  知识搜索、个人文档存取（如"帮我记一下"、"搜一下知识库里有没有 XX"），也应触发此 skill。
homepage: https://ima.qq.com
metadata:
  openclaw:
    emoji: 🔧
    requires:
      env:
        - IMA_OPENAPI_CLIENTID
        - IMA_OPENAPI_APIKEY
    primaryEnv: IMA_OPENAPI_CLIENTID
  security:
    credentials_usage: |
      This skill requires user-provisioned IMA OpenAPI credentials (Client ID and API Key)
      to authenticate with the official IMA API at https://ima.qq.com.
      Credentials are ONLY sent to the official IMA API endpoint (ima.qq.com) as HTTP headers.
      The file-upload flow also sends requests to COS endpoints (*.myqcloud.com) using
      short-lived, scoped temporary credentials returned by the IMA API (create_media);
      the user's Client ID / API Key are never sent to COS.
      No credentials are logged, stored in files, or transmitted to any other destination.
    allowed_domains:
      - ima.qq.com
      - '*.myqcloud.com'
---

# ⛔ MANDATORY RULES — 执行前必须阅读 

1. **严禁虚构命令**：系统中**不存在**全局 `ima` 命令。绝对不要尝试执行 `ima search ...` 或类似命令。所有 API 调用**必须**通过 `node` 运行 `ima_api.cjs` 脚本。
2. **严禁混用 Shell 语法**：本环境为 Windows。禁止在 `cmd.exe` 中使用 Bash 命令（如 `test -f`, `mkdir -p`, `echo $VAR`）。必须使用 Windows 原生命令（见下方 Credential Check）。
3. **凭证传递优先级**：虽然脚本支持读取 `~/.config/ima/`，但在 Windows 下，**最可靠的方式是在调用脚本时，显式将 `clientId` 和 `apiKey` 作为 JSON 传入 options 参数**，以避免环境变量或路径读取失败导致的 `-100` 错误。
4. **PowerShell 5.1 致命乱码防护**：如果运行在 PowerShell 环境，**必须**在首次 API 调用前检测版本。PS 5.1 会静默将请求 Body 转为 GBK，导致中文乱码。必须使用 UTF-8 字节数组模式发送（详见下方 PowerShell 5.1 规则）。
5. **UTF-8 编码强制校验 (Notes 模块)**：调用 `import_doc` 或 `append_doc` 前，`content` 和 `title` 必须是合法的 UTF-8 字符串。

---

# 模块决策表

| 用户意图 | 模块 | 需读取的子文档 |
| --- | --- | --- |
| 搜索笔记、浏览笔记本、获取笔记内容、创建笔记、追加内容 | `notes` | `notes/SKILL.md` |
| 上传文件、添加网页链接、搜索知识库、浏览知识库内容、获取知识库信息、获取可添加的知识库列表 | `knowledge-base` | `knowledge-base/SKILL.md` |
| 查看原文、分析原文、导出原文（需要 media_id） | `knowledge-base` | `knowledge-base/SKILL.md` |

## ⚠️ 易混淆场景
| 用户说的 | 实际意图 | 正确路由 |
| --- | --- | --- |
| "把这段内容添加到知识库 XX 里的笔记 YY" | 往已有 **笔记** 追加内容 | `notes` — 先搜索笔记获取 `note_id`，再用 `append_doc` |
| "把这个写到 XX 笔记里"、"记到 XX 笔记" | 往已有 **笔记** 追加内容 | `notes` — `append_doc` |
| "把这篇笔记添加到知识库" | 将笔记关联到 **知识库** | `knowledge-base` — `add_knowledge` with `media_type=11` |
| "上传文件到知识库" | 上传 **文件** 到知识库 | `knowledge-base` — `create_media` → COS → `add_knowledge` |
| "新建一篇笔记记录这些内容" | 创建 **新笔记** | `notes` — `import_doc` |
| "帮我记一下"、"记录一下"（未指定已有笔记） | 意图不明确，需要确认 | `notes` — 先询问用户是创建新笔记还是追加到哪篇已有笔记 |

## ⚠️ 跨模块任务 — 必须读取两个子模块
如果用户意图同时涉及「笔记」和「知识库」，必须先读取两个模块的 `SKILL.md` 再按顺序操作。
- "把知识库里的 XX 内容记到笔记" → 先读 `knowledge-base/SKILL.md` (搜索) → 再读 `notes/SKILL.md` (创建/追加)
- "查看原文"（知识库中的笔记类型媒体） → 先读 `knowledge-base/SKILL.md` (`get_media_info`) → 再读 `notes/SKILL.md` (`get_doc_content`)

---

# Credential Check (Windows 专属)

在执行任何 API 调用前，Agent 必须验证凭证是否可用。请选择当前 Shell 对应的命令进行检查：

**如果在 PowerShell 中:**
```powershell
if ((Test-Path "$env:USERPROFILE\.config\ima\client_id") -and (Test-Path "$env:USERPROFILE\.config\ima\api_key")) {
    Write-Host "✅ Credentials configured"
} else {
    Write-Host "⚠️ NO CREDENTIALS — setup required before any API call"
}
```

**如果在 CMD 中:**
```cmd
if exist "%USERPROFILE%\.config\ima\client_id" if exist "%USERPROFILE%\.config\ima\api_key" (
    echo ✅ Credentials configured
) else (
    echo ⚠️ NO CREDENTIALS — setup required before any API call
)
```

**如果 ⚠️ NO CREDENTIALS**：立即停止，引导用户：
1. 打开 https://ima.qq.com/agent-interface 获取 Client ID 和 API Key。
2. 运行以下 PowerShell 命令配置（推荐）：
   ```powershell
   New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\.config\ima"
   "你的_Client_ID" | Out-File -FilePath "$env:USERPROFILE\.config\ima\client_id" -Encoding utf8
   "你的_API_Key" | Out-File -FilePath "$env:USERPROFILE\.config\ima\api_key" -Encoding utf8
   ```

---

# API 调用模板 (Windows 专属)

所有请求统一为 HTTP POST + JSON Body。`ima_api` 已抽离到脚本：`ima_api.cjs`。
**核心原则**：使用 `node` 调用，并**显式传递凭证**到 options JSON 以确保 100% 成功。

### 方案 A：PowerShell 调用模板 (推荐，最安全)
```powershell
# 1. 获取脚本绝对路径 (假设当前在 skill 目录下，否则请替换为绝对路径)
$ScriptPath = "$PSScriptRoot\ima_api.cjs" # 或 "C:\Users\YourName\.qwenpaw\skills\ima-skill\ima_api.cjs"

# 2. 准备参数 (使用单引号包裹 JSON，避免 CMD/转义问题)
$ApiPath = "openapi/list_docs"
$Body = '{"limit":10}'
$Opts = '{"clientId":"你的_Client_ID", "apiKey":"你的_API_Key"}'

# 3. 执行并捕获错误
try {
    $resp = node $ScriptPath $ApiPath $Body $Opts
    Write-Host "Success: $resp"
} catch {
    Write-Host "Error: $_"
}
```

### 方案 B：CMD 调用模板
*(注意：CMD 中 JSON 内部的双引号必须使用 `\"` 转义)*
```cmd
node "C:\Users\YourName\.qwenpaw\skills\ima-skill\ima_api.cjs" "openapi/list_docs" "{\"limit\":10}" "{\"clientId\":\"你的_Client_ID\", \"apiKey\":\"你的_API_Key\"}"
```

---

# 错误处理 (两层检查)

1. **第一层 — 脚本执行错误** (进程非 0 退出，错误输出在 `stderr`)：
   - `-100`：程序错误（缺少凭证、参数非法、网络错误等），`msg` 可直接展示给用户。
   - `-200`：skill 需要更新。原请求未发送，`stdout` 中有更新上下文 JSON。需根据 `instruction` 引导用户更新后重试。
2. **第二层 — 后端业务错误** (进程正常退出，响应在 `stdout`)：
   - `stdout` 返回 JSON `{"code": 0, "msg": "...", "data": {...}}`
   - `code=0` 表示成功，从 `data` 提取业务字段。
   - `code≠0` 表示后端业务错误（如参数不合法、权限不足），直接将 `msg` 展示给用户。

---

# Detailed Rules Reference

## 1. PowerShell 5.1 Environment Detection (CRITICAL)
**此问题极其隐蔽**：PowerShell 5.1 下 `Invoke-RestMethod` 会静默将请求 Body 从 UTF-8 转为系统 ANSI 编码（中文 Windows 为 GBK），即使设置了 `Content-Type: charset=utf-8` 也无效。结果是请求看起来发送成功，但服务端收到的内容已经是乱码。

**当 Agent 运行在 PowerShell 环境时，必须在首次 API 调用前检测版本并采用安全模板：**

```powershell
# 1. 检测 PowerShell 版本
if ($PSVersionTable.PSVersion.Major -le 5) {
    Write-Host "⚠️ 检测到 PowerShell 5.1，将使用 UTF-8 字节数组模式发送请求"
    $useUtf8Bytes = $true
} else {
    Write-Host "✅ PowerShell 7+，默认 UTF-8，无需额外处理"
    $useUtf8Bytes = $false
}

# 2. 构建 Body (使用 ConvertTo-Json 避免手动拼接转义风险)
$bodyObj = @{ title = "标题"; content = "内容"; content_format = 1 }
$bodyJson = $bodyObj | ConvertTo-Json -Depth 10

# 3. 发送请求
$headers = @{
    "ima-openapi-clientid" = "你的_Client_ID"
    "ima-openapi-apikey" = "你的_API_Key"
    "Content-Type" = "application/json; charset=utf-8"
}

if ($useUtf8Bytes) {
    # CRITICAL: 必须转为字节数组，否则中文/非 ASCII 内容会变成乱码
    $utf8Bytes = [System.Text.Encoding]::UTF8.GetBytes($bodyJson)
    Invoke-RestMethod -Uri "https://ima.qq.com/openapi/xxx" -Method Post -Body $utf8Bytes -Headers $headers
} else {
    # PowerShell 7+ 可直接传字符串
    Invoke-RestMethod -Uri "https://ima.qq.com/openapi/xxx" -Method Post -Body $bodyJson -Headers $headers
}
```
*总结：在 PowerShell 5.1 环境中，所有 API 调用都必须将 Body 显式转为 UTF-8 字节数组。不检测版本直接发请求 = 中文内容必乱码。*

## 2. Detailed UTF-8 Encoding Rules (Notes 模块写入前)
每次调用 notes 写入类 API（`import_doc`/`append_doc`）之前，必须对 `content`、`title` 等所有字符串字段执行 UTF-8 编码校验/转换。

**Windows 环境转码方法推荐：**
- **Node.js (最推荐，与脚本环境一致)**:
  ```javascript
  // 如果内容已在变量中，清洗非法 UTF-8 字节
  const safeContent = Buffer.from(rawContent, 'utf8').toString('utf8');
  ```
- **Python (如果 Agent 调用外部脚本)**:
  ```python
  import sys
  # 读取并自动尝试转码为 UTF-8
  data = open('tmpfile', 'rb').read()
  for enc in ['utf-8', 'gbk', 'gb2312', 'big5', 'latin-1']:
      try:
          sys.stdout.write(data.decode(enc))
          break
      except (UnicodeDecodeError, LookupError):
          continue
  ```
- **PowerShell**:
  ```powershell
  $content = [System.IO.File]::ReadAllText('tmpfile', [System.Text.Encoding]::Default)
  [System.IO.File]::WriteAllText('tmpfile.utf8', $content, [System.Text.Encoding]::UTF8)
  ```
