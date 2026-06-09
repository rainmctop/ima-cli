# IMA CLI

A Rust-based command-line tool for IMA OpenAPI, providing knowledge base and notes management capabilities.

## Features

- **Knowledge Base Management**
  - Upload files to knowledge base (with automatic type detection and size validation)
  - Add URLs/web pages to knowledge base
  - Search and browse knowledge bases
  - Check for duplicate file names
  - Get media information

- **Notes Management**
  - Create new notes/documents
  - Append content to existing notes
  - Search and browse notes
  - Get note content

- **Credential Management**
  - Automatic credential loading from config file (`~/.config/ima/config.toml`)
  - Legacy support for separate `client_id` and `api_key` files
  - Environment variable support (`IMA_CLIENT_ID`, `IMA_API_KEY`)
  - Command-line option override

- **File Upload**
  - Save to local temporary file before upload (for large files)
  - Automatic content type detection
  - File size validation against API limits
  - COS (Cloud Object Storage) upload with temporary credentials

- **Error Handling**
  - Structured error output with error codes
  - Human-readable error messages in Chinese
  - Exit codes: 0 (success), 1 (programmatic error), 2 (update available)

## Installation

### From GitHub Releases (Windows)

1. Download the latest `ima-cli-windows-x64.exe` from [Releases](https://github.com/ima-skill/ima-cli/releases)
2. Rename to `ima.exe` (optional)
3. Place in a directory in your PATH

### Build from Source

```bash
# Clone the repository
git clone https://github.com/ima-skill/ima-cli.git
cd ima-cli

# Build release binary
cargo build --release

# Binary will be at target/release/ima (or ima.exe on Windows)
```

### Cross-Platform Builds via GitHub Actions

The repository includes GitHub Actions workflows that automatically build binaries for:
- Windows x64 (`ima.exe`)
- Linux x64 (`ima`)
- macOS x64 (`ima`)
- macOS ARM64 (`ima`)

## Configuration

### Method 1: Config File (Recommended)

Create `~/.config/ima/config.toml`:

```toml
client_id = "your_client_id"
api_key = "your_api_key"
base_url = "https://ima.qq.com"
```

### Method 2: Legacy Files

Create separate files:
- `~/.config/ima/client_id` - containing your Client ID
- `~/.config/ima/api_key` - containing your API Key

### Method 3: Environment Variables

```bash
# Windows (PowerShell)
$env:IMA_CLIENT_ID="your_client_id"
$env:IMA_API_KEY="your_api_key"

# Windows (CMD)
set IMA_CLIENT_ID=your_client_id
set IMA_API_KEY=your_api_key

# Linux/macOS
export IMA_CLIENT_ID="your_client_id"
export IMA_API_KEY="your_api_key"
```

### Priority Order

Credentials are loaded in this order (highest priority first):
1. Command-line options (explicit passing)
2. Environment variables
3. Config file (`config.toml`)
4. Legacy files (`client_id`, `api_key`)

## Usage

### Knowledge Base Commands

```bash
# Get knowledge base information
ima kb info --ids "kb_id1,kb_id2"

# List knowledge in a knowledge base
ima kb list --kb-id <knowledge_base_id> [--folder-id <folder_id>] [--limit 20] [--cursor ""]

# Search knowledge in a knowledge base
ima kb search --kb-id <knowledge_base_id> --query "<search_term>" [--cursor ""]

# Search knowledge bases
ima kb search-kb --query "<search_term>" [--limit 20] [--cursor ""]

# Get list of addable knowledge bases
ima kb addable [--limit 50] [--cursor ""]

# Upload a file to knowledge base
ima kb upload --file <path_to_file> --kb-id <knowledge_base_id> [--folder-id <folder_id>] [--content-type <mime>] [--title <title>]

# Import URLs to knowledge base
ima kb import-urls --kb-id <knowledge_base_id> --folder-id <folder_id> <url1> <url2> ...

# Check for repeated file names
ima kb check-repeated --kb-id <knowledge_base_id> [--folder-id <folder_id>] "file1.pdf:1" "file2.docx:3"

# Get media info
ima kb media-info --media-id <media_id>
```

### Notes Commands

```bash
# List documents in a notebook
ima notes list-docs --notebook-id <notebook_id> [--limit 20] [--cursor ""]

# Get document content
ima notes get-doc --doc-id <doc_id>

# Import/create a new document
ima notes import-doc --notebook-id <notebook_id> --title "<title>" --content "<content>" [--format 1]

# Append content to existing document
ima notes append-doc --doc-id <doc_id> --content "<content>" [--format 1]

# Search documents
ima notes search --notebook-id <notebook_id> --query "<search_term>" [--limit 20] [--cursor ""]
```

### Utility Commands

```bash
# Check for skill updates
ima check-update

# Force check for updates
ima --force-update-check check-update

# Legacy API call (compatible with ima_api.cjs)
ima api <api_path> '<json_body>' '<json_options>'
```

## Examples

### Upload a PDF to Knowledge Base

```powershell
# PowerShell
ima kb upload --file "C:\Documents\report.pdf" --kb-id "kb_12345"
```

```cmd
:: CMD
ima kb upload --file "C:\Documents\report.pdf" --kb-id "kb_12345"
```

### Add Web Pages to Knowledge Base

```powershell
ima kb import-urls --kb-id "kb_12345" --folder-id "kb_12345" "https://example.com/article1" "https://example.com/article2"
```

### Create a New Note

```powershell
ima notes import-doc --notebook-id "notebook_123" --title "Meeting Notes" --content "# Meeting Notes\n\n- Item 1\n- Item 2"
```

### Search Knowledge Base

```powershell
ima kb search --kb-id "kb_12345" --query "machine learning"
```

## Error Codes

| Code | Description |
|------|-------------|
| 0 | Success |
| -100 | Programmatic error (bad args, missing credentials, network, etc.) |
| -200 | Update available |
| 110001 | Invalid parameters |
| 110002 | Invalid configuration |
| 110010 | Network error |
| 110011 | Downstream logic error |
| 110020 | Security violation |
| 110021 | Rate limited |
| 110030 | Permission denied |

## File Size Limits

| File Type | Media Type | Max Size |
|-----------|------------|----------|
| Excel, TXT, Xmind, Markdown | 5/13/14/7 | 10 MB |
| Image | 9 | 30 MB |
| PDF, Word, PPT, Audio, etc. | 1/3/4/15 | 200 MB |

## Supported File Types

| Extension | Media Type | Content Type |
|-----------|------------|--------------|
| .pdf | 1 | application/pdf |
| .doc, .docx | 3 | application/msword |
| .ppt, .pptx | 4 | application/vnd.ms-powerpoint |
| .xls, .xlsx, .csv | 5 | application/vnd.ms-excel |
| .md, .markdown | 7 | text/markdown |
| .png, .jpg, .jpeg, .webp | 9 | image/* |
| .txt | 13 | text/plain |
| .xmind | 14 | application/x-xmind |
| .mp3, .m4a, .wav, .aac | 15 | audio/* |

## Building for Windows

The GitHub Actions workflow automatically builds Windows binaries when you push to the main branch or create a pull request.

To build locally on Windows:

```powershell
# Install Rust (if not already installed)
winget install Rustlang.Rust.MSVC

# Build release binary
cargo build --release

# Binary will be at target\release\ima.exe
```

## License

MIT License

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
