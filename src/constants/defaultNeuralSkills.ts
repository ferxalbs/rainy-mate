import { SkillManifest } from "../services/tauri";
import { getToolAirlockLevel } from "./toolPolicy";

export const DEFAULT_NEURAL_SKILLS: SkillManifest[] = [
  {
    name: "filesystem",
    version: "1.0.0",
    methods: [
      {
        name: "read_file",
        description: "Read file content",
        airlockLevel: getToolAirlockLevel("read_file"),
        parameters: {
          path: {
            type: "string",
            description: "Path to file",
            required: true,
          },
        },
      },
      {
        name: "list_files",
        description: "List files in a directory",
        airlockLevel: getToolAirlockLevel("list_files"),
        parameters: {
          path: {
            type: "string",
            description: "Directory path",
            required: true,
          },
        },
      },
      {
        name: "file_exists",
        description: "Check whether a path exists",
        airlockLevel: getToolAirlockLevel("file_exists"),
        parameters: {
          path: {
            type: "string",
            description: "Path to check",
            required: true,
          },
        },
      },
      {
        name: "get_file_info",
        description: "Get metadata for a file or directory",
        airlockLevel: getToolAirlockLevel("get_file_info"),
        parameters: {
          path: {
            type: "string",
            description: "Path to inspect",
            required: true,
          },
        },
      },
      {
        name: "search_files",
        description: "Search files by query",
        airlockLevel: getToolAirlockLevel("search_files"),
        parameters: {
          query: {
            type: "string",
            description: "Search query (regex supported)",
            required: true,
          },
          path: {
            type: "string",
            description: "Root path to search",
            required: false,
          },
          search_content: {
            type: "boolean",
            description: "Search within file contents",
            required: false,
          },
        },
      },
      {
        name: "read_file_chunk",
        description: "Read part of a file by offset",
        airlockLevel: getToolAirlockLevel("read_file_chunk"),
        parameters: {
          path: {
            type: "string",
            description: "Path to file",
            required: true,
          },
          offset: {
            type: "number",
            description: "Byte offset to start reading",
            required: false,
          },
          length: {
            type: "number",
            description: "Max bytes to read",
            required: false,
          },
        },
      },
      {
        name: "write_file",
        description: "Write content to file",
        airlockLevel: getToolAirlockLevel("write_file"),
        parameters: {
          path: {
            type: "string",
            description: "Path to write",
            required: true,
          },
          content: {
            type: "string",
            description: "Content to write",
            required: true,
          },
        },
      },
      {
        name: "mkdir",
        description: "Create directory",
        airlockLevel: getToolAirlockLevel("mkdir"),
        parameters: {
          path: {
            type: "string",
            description: "Directory path",
            required: true,
          },
        },
      },
      {
        name: "delete_file",
        description: "Delete file or directory",
        airlockLevel: getToolAirlockLevel("delete_file"),
        parameters: {
          path: {
            type: "string",
            description: "Path to delete",
            required: true,
          },
        },
      },
      {
        name: "move_file",
        description: "Move or rename file",
        airlockLevel: getToolAirlockLevel("move_file"),
        parameters: {
          source: {
            type: "string",
            description: "Source path",
            required: true,
          },
          destination: {
            type: "string",
            description: "Destination path",
            required: true,
          },
        },
      },
      {
        name: "append_file",
        description: "Append content to file",
        airlockLevel: getToolAirlockLevel("append_file"),
        parameters: {
          path: {
            type: "string",
            description: "Path to file",
            required: true,
          },
          content: {
            type: "string",
            description: "Content to append",
            required: true,
          },
        },
      },
    ],
  },
  {
    name: "shell",
    version: "1.0.0",
    methods: [
      {
        name: "execute_command",
        description: "Execute a shell command",
        airlockLevel: getToolAirlockLevel("execute_command"),
        parameters: {
          command: {
            type: "string",
            description: "Command to execute (whitelisted)",
            required: true,
          },
          args: {
            type: "array",
            description: "Command arguments",
            required: true,
          },
        },
      },
      {
        name: "git_status",
        description: "Get git status with stable wrapper",
        airlockLevel: getToolAirlockLevel("git_status"),
        parameters: {
          path: {
            type: "string",
            description: "Optional directory/file path inside workspace",
            required: false,
          },
          short: {
            type: "boolean",
            description: "Compact output",
            required: false,
          },
        },
      },
      {
        name: "git_diff",
        description: "Get git diff with stable wrapper",
        airlockLevel: getToolAirlockLevel("git_diff"),
        parameters: {
          path: {
            type: "string",
            description: "Optional directory/file path inside workspace",
            required: false,
          },
          staged: {
            type: "boolean",
            description: "Show staged diff",
            required: false,
          },
        },
      },
    ],
  },
  {
    name: "web",
    version: "1.0.0",
    methods: [
      {
        name: "web_search",
        description: "Search the web",
        airlockLevel: getToolAirlockLevel("web_search"),
        parameters: {
          query: {
            type: "string",
            description: "Search query",
            required: true,
          },
        },
      },
      {
        name: "read_web_page",
        description: "Read a web page",
        airlockLevel: getToolAirlockLevel("read_web_page"),
        parameters: {
          url: {
            type: "string",
            description: "URL to read",
            required: true,
          },
        },
      },
      {
        name: "http_get_json",
        description: "Fetch JSON from HTTP(S) APIs",
        airlockLevel: getToolAirlockLevel("http_get_json"),
        parameters: {
          url: {
            type: "string",
            description: "HTTP(S) URL",
            required: true,
          },
          timeout_ms: {
            type: "number",
            description: "Timeout in milliseconds",
            required: false,
          },
          max_bytes: {
            type: "number",
            description: "Maximum response size in bytes",
            required: false,
          },
        },
      },
      {
        name: "http_post_json",
        description: "POST JSON to HTTP(S) APIs",
        airlockLevel: getToolAirlockLevel("http_post_json"),
        parameters: {
          url: {
            type: "string",
            description: "HTTP(S) URL",
            required: true,
          },
          body: {
            type: "object",
            description: "JSON body",
            required: true,
          },
          timeout_ms: {
            type: "number",
            description: "Timeout in milliseconds",
            required: false,
          },
          max_bytes: {
            type: "number",
            description: "Maximum response size in bytes",
            required: false,
          },
        },
      },
    ],
  },
  {
    name: "browser",
    version: "1.0.0",
    methods: [
      {
        name: "browse_url",
        description: "Open a URL in the browser",
        airlockLevel: getToolAirlockLevel("browse_url"),
        parameters: {
          url: {
            type: "string",
            description: "URL to open",
            required: true,
          },
        },
      },
      {
        name: "open_new_tab",
        description: "Open a URL in a new tab",
        airlockLevel: getToolAirlockLevel("open_new_tab"),
        parameters: {
          url: {
            type: "string",
            description: "URL to open",
            required: true,
          },
        },
      },
      {
        name: "click_element",
        description: "Click an element by CSS selector",
        airlockLevel: getToolAirlockLevel("click_element"),
        parameters: {
          selector: {
            type: "string",
            description: "CSS selector",
            required: true,
          },
        },
      },
      {
        name: "wait_for_selector",
        description: "Wait until a selector is present",
        airlockLevel: getToolAirlockLevel("wait_for_selector"),
        parameters: {
          selector: {
            type: "string",
            description: "CSS selector",
            required: true,
          },
          timeout_ms: {
            type: "number",
            description: "Timeout in milliseconds",
            required: false,
          },
        },
      },
      {
        name: "type_text",
        description: "Type text into an element",
        airlockLevel: getToolAirlockLevel("type_text"),
        parameters: {
          selector: {
            type: "string",
            description: "CSS selector",
            required: true,
          },
          text: {
            type: "string",
            description: "Text to type",
            required: true,
          },
          clear_first: {
            type: "boolean",
            description: "Clear previous value before typing",
            required: false,
          },
        },
      },
      {
        name: "go_back",
        description: "Navigate one step back in history",
        airlockLevel: getToolAirlockLevel("go_back"),
        parameters: {
          wait_ms: {
            type: "number",
            description: "Wait time after going back",
            required: false,
          },
        },
      },
      {
        name: "screenshot",
        description: "Take a screenshot of the current page",
        airlockLevel: getToolAirlockLevel("screenshot"),
        parameters: {},
      },
      {
        name: "get_page_content",
        description: "Get HTML content of the current page",
        airlockLevel: getToolAirlockLevel("get_page_content"),
        parameters: {},
      },
      {
        name: "extract_links",
        description: "Extract links from the current page",
        airlockLevel: getToolAirlockLevel("extract_links"),
        parameters: {
          limit: {
            type: "number",
            description: "Maximum number of links to return",
            required: false,
          },
        },
      },
      {
        name: "submit_form",
        description: "Submit a form in the current page",
        airlockLevel: getToolAirlockLevel("submit_form"),
        parameters: {
          form_selector: {
            type: "string",
            description: "Optional form selector",
            required: false,
          },
          submit_selector: {
            type: "string",
            description: "Optional submit button selector",
            required: false,
          },
          wait_ms: {
            type: "number",
            description: "Wait time after submit",
            required: false,
          },
        },
      },
    ],
  },
];
