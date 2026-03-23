use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct ReadFileArgs {
    /// The path to the file to read
    pub path: String,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct IngestDocumentArgs {
    /// The path to the document to ingest (PDF, Markdown, or plaintext)
    pub path: String,
    /// Optional tags to attach to the ingested memory
    pub tags: Option<Vec<String>>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct ReadManyFilesArgs {
    /// Paths to read as UTF-8 text
    pub paths: Vec<String>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct WriteFileArgs {
    /// The path where the file should be written
    pub path: String,
    /// The content to write to the file
    pub content: String,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct ListFilesArgs {
    /// The directory path to list
    pub path: String,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct FileExistsArgs {
    /// The file or directory path to check
    pub path: String,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct FileInfoArgs {
    /// The file or directory path to inspect
    pub path: String,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct ReadFileChunkArgs {
    /// The file path to read from
    pub path: String,
    /// Byte offset to start reading from
    pub offset: Option<u64>,
    /// Maximum number of bytes to read (defaults to 8192, max 65536)
    pub length: Option<usize>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct MakeDirArgs {
    /// The directory path to create
    pub path: String,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct DeleteFileArgs {
    /// The path to the file or directory to delete
    pub path: String,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct MoveFileArgs {
    /// The source path
    pub source: String,
    /// The destination path
    pub destination: String,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct SearchFilesArgs {
    /// The regex query to search for
    pub query: String,
    /// The root path to start searching from
    pub path: Option<String>,
    /// Whether to search file content (default: true). If false, only file names are matched.
    pub search_content: Option<bool>,
    /// Whether regex matching is case-sensitive (default: false)
    pub case_sensitive: Option<bool>,
    /// Maximum filesystem entries to scan (default: 2000, max: 20000)
    pub max_files: Option<usize>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct ExecuteCommandArgs {
    /// The command to execute (e.g., npm, cargo, git)
    pub command: String,
    /// Arguments for the command
    pub args: Vec<String>,
    /// Optional timeout in milliseconds (default: 120000, max: 600000)
    pub timeout_ms: Option<u64>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct GitStatusArgs {
    /// Optional workspace path used as git working directory
    pub path: Option<String>,
    /// Use compact output format
    pub short: Option<bool>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct GitDiffArgs {
    /// Optional workspace path used as git working directory
    pub path: Option<String>,
    /// Show staged diff instead of unstaged
    pub staged: Option<bool>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct GitLogArgs {
    /// Optional workspace path used as git working directory
    pub path: Option<String>,
    /// Maximum number of commits (default 20, max 100)
    pub max_count: Option<u32>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct GitShowArgs {
    /// Optional workspace path used as git working directory
    pub path: Option<String>,
    /// Git object spec (commit, tag, or file ref), defaults to HEAD
    pub target: Option<String>,
    /// Optional maximum lines to include in output (default 300, max 2000)
    pub max_lines: Option<u32>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct GitBranchListArgs {
    /// Optional workspace path used as git working directory
    pub path: Option<String>,
    /// Include remote branches (default true)
    pub include_remote: Option<bool>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct WebSearchArgs {
    /// The query to search for
    pub query: String,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct ReadWebPageArgs {
    /// The URL to read
    pub url: String,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct HttpPostJsonArgs {
    /// URL to POST to (http/https only)
    pub url: String,
    /// JSON body to send
    pub body: Value,
    /// Request timeout in milliseconds (default: 15000)
    pub timeout_ms: Option<u64>,
    /// Maximum allowed response size in bytes (default: 512KB, max: 2MB)
    pub max_bytes: Option<usize>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct BrowserNavigateArgs {
    /// URL to navigate to
    pub url: String,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct BrowserClickArgs {
    /// CSS selector to click
    pub selector: String,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct WaitForSelectorArgs {
    /// CSS selector to wait for
    pub selector: String,
    /// Timeout in milliseconds (default: 10000, max: 60000)
    pub timeout_ms: Option<u64>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct TypeTextArgs {
    /// CSS selector of the input/textarea/contenteditable element
    pub selector: String,
    /// Text to type
    pub text: String,
    /// Whether to clear existing value first
    pub clear_first: Option<bool>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct SubmitFormArgs {
    /// Optional form selector; defaults to first form element
    pub form_selector: Option<String>,
    /// Optional submit control selector to click directly
    pub submit_selector: Option<String>,
    /// Wait time after submit in milliseconds (default: 1200, max: 10000)
    pub wait_ms: Option<u64>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct GoBackArgs {
    /// Wait time after navigation in milliseconds (default: 1000, max: 10000)
    pub wait_ms: Option<u64>,
}

#[derive(JsonSchema, Serialize, Deserialize, Default)]
pub struct ExtractLinksArgs {
    /// Maximum number of links to return
    pub limit: Option<usize>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct HttpGetJsonArgs {
    /// URL to fetch (http/https only)
    pub url: String,
    /// Request timeout in milliseconds (default: 15000)
    pub timeout_ms: Option<u64>,
    /// Maximum allowed response size in bytes (default: 512KB, max: 2MB)
    pub max_bytes: Option<usize>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct HttpGetTextArgs {
    /// URL to fetch as text (http/https only)
    pub url: String,
    /// Request timeout in milliseconds (default: 15000)
    pub timeout_ms: Option<u64>,
    /// Maximum allowed response size in bytes (default: 512KB, max: 2MB)
    pub max_bytes: Option<usize>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct ListFilesDetailedArgs {
    /// Directory path to list (default ".")
    pub path: Option<String>,
    /// Include entries whose names start with "."
    pub include_hidden: Option<bool>,
    /// Maximum number of entries to return (default 200, max 2000)
    pub limit: Option<usize>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct SaveMemoryArgs {
    /// The fact or knowledge to persist (e.g. "User's name is Fernando")
    pub content: String,
    /// Optional descriptive tags for retrieval (e.g. ["user", "preference", "name"])
    pub tags: Option<Vec<String>>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct RecallMemoryArgs {
    /// Natural language query to search long-term memory
    pub query: String,
    /// Maximum number of results to return (default 5, max 20)
    pub limit: Option<u64>,
}

// ── IRONMILL — Document Generation Args (KINGFALL Phase 1) ──────────────────

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct PdfSection {
    /// Section heading (will be rendered as bold heading)
    #[schemars(length(max = 200))]
    pub heading: Option<String>,
    /// Body text for this section
    #[schemars(length(min = 1, max = 65535))]
    pub body: String,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct PdfCreateArgs {
    /// Output file name (e.g. "report.pdf"). Resolved inside the workspace.
    #[schemars(length(min = 1, max = 255))]
    pub filename: String,
    /// Optional document title shown in metadata
    #[schemars(length(max = 200))]
    pub title: Option<String>,
    /// Sections to include in the PDF
    #[schemars(length(min = 1, max = 100))]
    pub sections: Vec<PdfSection>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct PdfReadArgs {
    /// Path to the PDF file to read
    #[schemars(length(min = 1, max = 4096))]
    pub path: String,
    /// Maximum number of pages to extract (default: all, max: 200)
    #[schemars(range(min = 1, max = 200))]
    pub max_pages: Option<usize>,
}

#[derive(JsonSchema, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum ExcelCell {
    /// Plain text value
    Text(String),
    /// Numeric value
    Number(f64),
    /// Boolean value
    Bool(bool),
    /// Excel formula (e.g. "=SUM(A1:A10)")
    Formula(String),
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct ExcelSheet {
    /// Sheet tab name
    #[schemars(length(min = 1, max = 31))]
    pub name: String,
    /// Header row (column labels)
    #[schemars(length(max = 100), inner(length(min = 1, max = 255)))]
    pub headers: Option<Vec<String>>,
    /// Data rows — each row is a Vec of ExcelCell
    #[schemars(length(max = 10000), inner(length(max = 100)))]
    pub rows: Vec<Vec<ExcelCell>>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct ExcelWriteArgs {
    /// Output file name (e.g. "data.xlsx"). Resolved inside the workspace.
    #[schemars(length(min = 1, max = 255))]
    pub filename: String,
    /// Sheets to write. At least one sheet is required.
    #[schemars(length(min = 1, max = 20))]
    pub sheets: Vec<ExcelSheet>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct ExcelReadArgs {
    /// Path to the .xlsx / .xls / .ods file to read
    #[schemars(length(min = 1, max = 4096))]
    pub path: String,
    /// Maximum rows per sheet to return (default: 1000, max: 10000)
    #[schemars(range(min = 1, max = 10000))]
    pub max_rows: Option<usize>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct DocxParagraph {
    /// Text content of this paragraph
    #[schemars(length(min = 1, max = 32768))]
    pub text: String,
    /// Heading level: 1–6. Omit for normal body text.
    #[schemars(range(min = 1, max = 6))]
    pub heading_level: Option<u8>,
    /// Bold text
    pub bold: Option<bool>,
    /// Italic text
    pub italic: Option<bool>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct DocxCreateArgs {
    /// Output file name (e.g. "document.docx"). Resolved inside the workspace.
    #[schemars(length(min = 1, max = 255))]
    pub filename: String,
    /// Document paragraphs in order
    #[schemars(length(min = 1, max = 200))]
    pub paragraphs: Vec<DocxParagraph>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct ArchiveCreateArgs {
    /// Output archive file name (e.g. "output.zip"). Resolved inside the workspace.
    #[schemars(length(min = 1, max = 255))]
    pub filename: String,
    /// Absolute paths of files to include in the archive
    #[schemars(length(min = 1, max = 100), inner(length(min = 1, max = 4096)))]
    pub files: Vec<String>,
}
