use super::args::*;
use super::SkillExecutor;
use crate::ai::provider_types::{FunctionDefinition, Tool};
use schemars::schema_for;

pub fn registered_tool_definitions() -> Vec<Tool> {
    vec![
        tool("read_file", "Read the contents of a file", schema_for!(ReadFileArgs)),
        tool("ingest_document", "Ingest a document (PDF, Text, Markdown) into the semantic workspace memory", schema_for!(IngestDocumentArgs)),
        tool(
            "read_many_files",
            "Read multiple UTF-8 text files in one call",
            schema_for!(ReadManyFilesArgs),
        ),
        tool("write_file", "Write content to a file", schema_for!(WriteFileArgs)),
        tool(
            "append_file",
            "Append content to a file",
            schema_for!(WriteFileArgs),
        ),
        tool("list_files", "List files in a directory", schema_for!(ListFilesArgs)),
        tool(
            "list_files_detailed",
            "List files with metadata (size, modified time, type)",
            schema_for!(ListFilesDetailedArgs),
        ),
        tool(
            "file_exists",
            "Check whether a file or directory exists",
            schema_for!(FileExistsArgs),
        ),
        tool(
            "get_file_info",
            "Get metadata (size, timestamps, type) for a file or directory",
            schema_for!(FileInfoArgs),
        ),
        tool(
            "search_files",
            "Search files by regex in names and (by default) text content",
            schema_for!(SearchFilesArgs),
        ),
        tool(
            "read_file_chunk",
            "Read a chunk of a text file by byte offset for large-file processing",
            schema_for!(ReadFileChunkArgs),
        ),
        tool(
            "execute_command",
            "Execute a shell command (npm, cargo, git, ls, grep)",
            schema_for!(ExecuteCommandArgs),
        ),
        tool(
            "git_status",
            "Get git status with stable wrapper options",
            schema_for!(GitStatusArgs),
        ),
        tool(
            "git_diff",
            "Get git diff with stable wrapper options",
            schema_for!(GitDiffArgs),
        ),
        tool(
            "git_log",
            "Get recent commit history with stable wrapper options",
            schema_for!(GitLogArgs),
        ),
        tool(
            "git_show",
            "Show details for a commit/tag/file with optional line limits",
            schema_for!(GitShowArgs),
        ),
        tool(
            "git_branch_list",
            "List local/remote git branches with commit summary",
            schema_for!(GitBranchListArgs),
        ),
        tool("web_search", "Search the web for information", schema_for!(WebSearchArgs)),
        tool(
            "http_get_json",
            "Fetch JSON from an HTTP(S) endpoint with timeout and size limits",
            schema_for!(HttpGetJsonArgs),
        ),
        tool(
            "http_get_text",
            "Fetch text/HTML from an HTTP(S) endpoint with timeout and size limits",
            schema_for!(HttpGetTextArgs),
        ),
        tool(
            "http_post_json",
            "POST JSON to an HTTP(S) endpoint with timeout and size limits",
            schema_for!(HttpPostJsonArgs),
        ),
        tool(
            "read_web_page",
            "Read the content of a web page using a headless scraper (good for static text)",
            schema_for!(ReadWebPageArgs),
        ),
        tool(
            "browse_url",
            "Open a URL in the visible browser (for dynamic sites). Returns title and content preview.",
            schema_for!(BrowserNavigateArgs),
        ),
        tool(
            "open_new_tab",
            "Open a URL in a new browser tab",
            schema_for!(BrowserNavigateArgs),
        ),
        tool(
            "click_element",
            "Click an element in the browser by CSS selector",
            schema_for!(BrowserClickArgs),
        ),
        tool(
            "wait_for_selector",
            "Wait until a CSS selector appears in the active page",
            schema_for!(WaitForSelectorArgs),
        ),
        tool(
            "type_text",
            "Type text into an input/textarea using selector, optionally clearing first",
            schema_for!(TypeTextArgs),
        ),
        tool("submit_form", "Submit a form in the active page", schema_for!(SubmitFormArgs)),
        tool(
            "go_back",
            "Navigate one step back in browser history",
            schema_for!(GoBackArgs),
        ),
        tool(
            "screenshot",
            "Take a screenshot of the current browser page",
            serde_json::json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "get_page_content",
            "Get the HTML content of the current browser page",
            serde_json::json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "get_page_snapshot",
            "Get URL/title/text preview from the current browser page",
            serde_json::json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "extract_links",
            "Extract clickable links from the current browser page (href + text)",
            schema_for!(ExtractLinksArgs),
        ),
        tool("mkdir", "Create a new directory", schema_for!(MakeDirArgs)),
        tool(
            "delete_file",
            "Delete a file or directory",
            schema_for!(DeleteFileArgs),
        ),
        tool(
            "move_file",
            "Move or rename a file or directory",
            schema_for!(MoveFileArgs),
        ),
        tool(
            "save_memory",
            "Persist a fact, preference, or user detail to long-term memory so it can be recalled in future sessions",
            schema_for!(SaveMemoryArgs),
        ),
        tool(
            "recall_memory",
            "Search long-term memory with a natural language query and return the most relevant stored facts",
            schema_for!(RecallMemoryArgs),
        ),
        // ── IRONMILL — Document Generation Tools (KINGFALL Phase 1) ──────────
        tool(
            "pdf_create",
            "Create a PDF document natively with titled sections. Bounded to 100 sections and workspace-scoped output.",
            schema_for!(PdfCreateArgs),
        ),
        tool(
            "pdf_read",
            "Extract text content from a PDF file, page by page. Returns structured page data with a 200-page hard cap.",
            schema_for!(PdfReadArgs),
        ),
        tool(
            "excel_write",
            "Create an Excel (.xlsx) spreadsheet with typed cells. Bounded to 20 sheets, 10k rows per sheet, and workspace-scoped output.",
            schema_for!(ExcelWriteArgs),
        ),
        tool(
            "excel_read",
            "Read an Excel (.xlsx / .xls / .ods) file and return structured rows with a 10k-row hard cap per sheet.",
            schema_for!(ExcelReadArgs),
        ),
        tool(
            "docx_create",
            "Create or overwrite a Word (.docx) document with paragraphs, headings, bold, and italic formatting. Bounded to 200 paragraphs.",
            schema_for!(DocxCreateArgs),
        ),
        tool(
            "docx_read",
            "Read a Word (.docx) document and extract paragraph text for revision workflows.",
            schema_for!(DocxReadArgs),
        ),
        tool(
            "archive_create",
            "Bundle workspace-scoped files into a .zip archive. Rejects directories and duplicate archive entry names.",
            schema_for!(ArchiveCreateArgs),
        ),
    ]
}

impl SkillExecutor {
    /// Get all available tools and their JSON schemas,
    /// including built-in tools, installed third-party Wasm skills,
    /// and connected MCP server tools.
    pub async fn get_tool_definitions(&self) -> Vec<Tool> {
        let mut tools = registered_tool_definitions();

        // Merge installed and enabled third-party Wasm skills into the tool list
        // so the LLM knows they exist and can call them.
        if let Ok(dynamic) = self.third_party_registry.dynamic_tool_definitions() {
            tools.extend(dynamic);
        }

        // Merge connected MCP server tools
        let mcp_tools = self.mcp_service.get_tools().await;
        tools.extend(mcp_tools);

        tools
    }
}

fn tool<S: serde::Serialize>(name: &str, description: &str, schema: S) -> Tool {
    Tool {
        r#type: "function".to_string(),
        function: FunctionDefinition {
            name: name.to_string(),
            description: description.to_string(),
            parameters: serde_json::to_value(schema).unwrap(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::tool_policy::get_tool_policy;

    #[test]
    fn every_registered_tool_has_explicit_policy_entry() {
        let tools = registered_tool_definitions();

        let missing: Vec<String> = tools
            .iter()
            .map(|tool| tool.function.name.clone())
            .filter(|name| get_tool_policy(name).is_none())
            .collect();

        assert!(
            missing.is_empty(),
            "Missing explicit tool policy entries for: {:?}",
            missing
        );
    }
}
