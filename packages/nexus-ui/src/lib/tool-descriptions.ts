/**
 * Smart formatter: turns raw tool names + args into human-readable descriptions.
 *
 * e.g. search_files({ pattern: "*.md", path: "/src" }) → "Search for *.md files in /src"
 *      read_file({ path: "/src/App.tsx" })             → "Read App.tsx"
 *      execute_shell({ command: "npm test" })           → "Run npm test"
 */

type Args = Record<string, unknown>;

function basename(path: string): string {
  const parts = path.split("/");
  return parts[parts.length - 1] || path;
}

function shortenPath(path: string, maxLen = 40): string {
  if (path.length <= maxLen) return path;
  const parts = path.split("/").filter(Boolean);
  if (parts.length <= 2) return path;
  return `.../${parts.slice(-2).join("/")}`;
}

function tryParseArgs(argsText?: string): Args | null {
  if (!argsText) return null;
  try {
    return JSON.parse(argsText) as Args;
  } catch {
    return null;
  }
}

type Formatter = (args: Args) => string | null;

const TOOL_FORMATTERS: Record<string, Formatter> = {
  // File operations
  read_file: (a) => {
    const p = a.path as string | undefined;
    return p ? `Read ${basename(p)}` : null;
  },
  write_file: (a) => {
    const p = a.path as string | undefined;
    return p ? `Write ${basename(p)}` : null;
  },
  edit_file: (a) => {
    const p = a.path as string | undefined;
    return p ? `Edit ${basename(p)}` : null;
  },
  create_file: (a) => {
    const p = a.path as string | undefined;
    return p ? `Create ${basename(p)}` : null;
  },
  delete_file: (a) => {
    const p = a.path as string | undefined;
    return p ? `Delete ${basename(p)}` : null;
  },

  // Search
  search_files: (a) => {
    const pattern = a.pattern as string | undefined;
    const path = a.path as string | undefined;
    if (pattern && path) return `Search for ${pattern} in ${shortenPath(path)}`;
    if (pattern) return `Search for ${pattern}`;
    return null;
  },
  search_content: (a) => {
    const pattern = a.pattern as string | undefined;
    const path = a.path as string | undefined;
    if (pattern && path) return `Search "${pattern}" in ${shortenPath(path)}`;
    if (pattern) return `Search "${pattern}"`;
    return null;
  },
  list_directory: (a) => {
    const p = a.path as string | undefined;
    return p ? `List ${shortenPath(p)}` : null;
  },

  // Shell / command execution
  execute_shell: (a) => {
    const cmd = a.command as string | undefined;
    return cmd ? `Run ${cmd}` : null;
  },
  execute_command: (a) => {
    const cmd = a.command as string | undefined;
    const args = a.args as string[] | undefined;
    if (cmd && args?.length) return `Run ${cmd} ${args.join(" ")}`;
    return cmd ? `Run ${cmd}` : null;
  },
  run_terminal_command: (a) => {
    const cmd = a.command as string | undefined;
    return cmd ? `Run ${cmd}` : null;
  },

  // Web
  web_search: (a) => {
    const q = a.query as string | undefined;
    return q ? `Search web: "${q}"` : null;
  },
  fetch_url: (a) => {
    const url = a.url as string | undefined;
    return url ? `Fetch ${url}` : null;
  },

  // Git
  git_diff: () => "View git diff",
  git_status: () => "Check git status",
  git_log: () => "View git log",
  git_commit: (a) => {
    const msg = a.message as string | undefined;
    return msg ? `Commit: "${msg}"` : "Create git commit";
  },

  // Nexus-specific
  nexus_read_file: (a) => {
    const p = a.path as string | undefined;
    return p ? `Read ${basename(p)}` : null;
  },
  nexus_write_file: (a) => {
    const p = a.path as string | undefined;
    return p ? `Write ${basename(p)}` : null;
  },
  nexus_edit_file: (a) => {
    const p = a.path as string | undefined;
    return p ? `Edit ${basename(p)}` : null;
  },
  nexus_search_files: (a) => {
    const pattern = a.pattern as string | undefined;
    return pattern ? `Search for ${pattern}` : null;
  },
  nexus_search_content: (a) => {
    const pattern = a.pattern as string | undefined;
    return pattern ? `Search "${pattern}"` : null;
  },
  nexus_list_directory: (a) => {
    const p = a.path as string | undefined;
    return p ? `List ${shortenPath(p)}` : null;
  },
  nexus_execute_command: (a) => {
    const cmd = a.command as string | undefined;
    const args = a.args as string[] | undefined;
    if (cmd && args?.length) return `Run ${cmd} ${args.join(" ")}`;
    return cmd ? `Run ${cmd}` : null;
  },
};

/**
 * Format a tool call into a human-readable description.
 *
 * @param toolName - raw tool name (e.g. "nexus_search_files")
 * @param argsText - JSON-encoded args string
 * @returns human-readable description
 */
export function formatToolDescription(
  toolName: string,
  argsText?: string,
): string {
  const args = tryParseArgs(argsText);

  // Try exact match
  const formatter = TOOL_FORMATTERS[toolName];
  if (formatter && args) {
    const result = formatter(args);
    if (result) return result;
  }

  // Try prefix-stripped match (e.g. "mcp_nexus_search_files" → "search_files")
  const stripped = toolName.replace(/^(?:mcp_|_nexus_|nexus_)/, "");
  const strippedFormatter = TOOL_FORMATTERS[stripped];
  if (strippedFormatter && args) {
    const result = strippedFormatter(args);
    if (result) return result;
  }

  // Fallback: humanize the tool name
  return humanizeToolName(toolName);
}

/** Convert snake_case tool name to readable text. */
function humanizeToolName(name: string): string {
  // Strip common prefixes
  const clean = name
    .replace(/^mcp__[^_]+__/, "")  // "mcp__nexus__search_files" → "search_files"
    .replace(/^nexus_/, "")
    .replace(/^_nexus_/, "");

  // snake_case → Title Case
  return clean
    .split("_")
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
}
