import type { ChatArtifact } from "../types/agent";

function isAbsolutePath(path: string): boolean {
  return path.startsWith("/") || /^[A-Za-z]:[\\/]/.test(path);
}

function normalizePath(path: string): string {
  return path.replace(/\\/g, "/");
}

function artifactFromPath(path: string, originTool: string): ChatArtifact | null {
  if (!isAbsolutePath(path)) return null;

  const filename = path.split(/[\\/]/).pop();
  const extension = filename?.split(".").pop()?.toLowerCase();
  if (!filename || !extension) return null;

  switch (extension) {
    case "png":
      return makeArtifact(path, filename, "image", "image/png", "inline", originTool);
    case "jpg":
    case "jpeg":
      return makeArtifact(path, filename, "image", "image/jpeg", "inline", originTool);
    case "gif":
      return makeArtifact(path, filename, "image", "image/gif", "inline", originTool);
    case "webp":
      return makeArtifact(path, filename, "image", "image/webp", "inline", originTool);
    case "pdf":
      return makeArtifact(
        path,
        filename,
        "pdf",
        "application/pdf",
        "preview",
        originTool,
      );
    case "docx":
      return makeArtifact(
        path,
        filename,
        "docx",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "system_default",
        originTool,
      );
    case "xlsx":
      return makeArtifact(
        path,
        filename,
        "xlsx",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "system_default",
        originTool,
      );
    default:
      return null;
  }
}

function makeArtifact(
  path: string,
  filename: string,
  kind: ChatArtifact["kind"],
  mimeType: string,
  openMode: ChatArtifact["openMode"],
  originTool: string,
): ChatArtifact {
  return {
    id: `${originTool}:${path}`,
    path,
    filename,
    kind,
    mimeType,
    openMode,
    availableActions: ["open"],
    originTool,
  };
}

function extractPathFromResult(result: string): string | null {
  try {
    const parsed = JSON.parse(result) as { path?: unknown };
    return typeof parsed.path === "string" ? parsed.path : null;
  } catch {
    return null;
  }
}

function extractPathFromArgs(args?: string): string | null {
  if (!args) return null;

  try {
    const parsed = JSON.parse(args) as { path?: unknown; filename?: unknown };
    if (typeof parsed.path === "string") return parsed.path;
    if (typeof parsed.filename === "string") return parsed.filename;
  } catch {
    return null;
  }

  return null;
}

export function artifactFromToolResult(
  toolName: string,
  args: string | undefined,
  result: string,
): ChatArtifact | null {
  const path = extractPathFromResult(result) ?? extractPathFromArgs(args);
  if (!path) return null;
  return artifactFromPath(path, toolName);
}

export function appendUniqueArtifact(
  artifacts: ChatArtifact[] | undefined,
  artifact: ChatArtifact,
): ChatArtifact[] {
  const current = artifacts ?? [];
  const normalized = normalizePath(artifact.path);
  if (current.some((existing) => normalizePath(existing.path) === normalized)) {
    return current;
  }
  return [...current, artifact];
}

export function isRenderableArtifactPath(path: string): boolean {
  return isAbsolutePath(path);
}
