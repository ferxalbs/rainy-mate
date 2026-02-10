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
    ],
  },
];
