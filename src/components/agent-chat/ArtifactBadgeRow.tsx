import React, { useMemo, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { FileImage, FileSpreadsheet, FileText, Eye, ExternalLink } from "lucide-react";

import * as tauri from "../../services/tauri";
import type { ChatArtifact } from "../../types/agent";
import { isRenderableArtifactPath } from "../../lib/chat-artifacts";
import { Button } from "../ui/button";
import { Card, CardContent } from "../ui/card";

function artifactIcon(kind: ChatArtifact["kind"]) {
  switch (kind) {
    case "image":
      return FileImage;
    case "xlsx":
      return FileSpreadsheet;
    default:
      return FileText;
  }
}

function actionLabel(openMode: ChatArtifact["openMode"]) {
  return openMode === "preview" ? "Preview" : "Open";
}

interface ArtifactBadgeRowProps {
  artifacts: ChatArtifact[];
}

// Memoized component to prevent re-renders on every message token chunk
export const ArtifactBadgeRow = React.memo(function ArtifactBadgeRow({ artifacts }: ArtifactBadgeRowProps) {
  const [openingPath, setOpeningPath] = useState<string | null>(null);
  const [errorByPath, setErrorByPath] = useState<Record<string, string>>({});

  const visibleArtifacts = useMemo(
    () =>
      artifacts.filter(
        (artifact) =>
          isRenderableArtifactPath(artifact.path) &&
          (artifact.kind === "image" || artifact.availableActions.includes("open")),
      ),
    [artifacts],
  );

  if (visibleArtifacts.length === 0) return null;

  return (
    <div className="flex w-full flex-col gap-2.5">
      {visibleArtifacts.map((artifact) => {
        const Icon = artifactIcon(artifact.kind);
        const imageSrc =
          artifact.kind === "image" ? convertFileSrc(artifact.path) : null;

        return (
          <div key={artifact.id} className="flex flex-col gap-1.5">
            <Card
              size="sm"
              className="w-full max-w-md border border-white/10 bg-white/40 py-0 backdrop-blur-md dark:bg-white/5"
            >
              <CardContent className="flex items-center gap-3 py-3">
                {artifact.kind === "image" && imageSrc ? (
                  <img
                    src={imageSrc}
                    alt={artifact.filename}
                    className="h-16 w-16 rounded-xl border border-white/10 object-cover"
                  />
                ) : (
                  <div className="flex h-12 w-12 items-center justify-center rounded-xl border border-white/10 bg-background/70">
                    <Icon className="size-5 text-muted-foreground" />
                  </div>
                )}

                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <span className="rounded-full border border-white/10 bg-background/70 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.16em] text-muted-foreground">
                      {artifact.kind}
                    </span>
                    <span className="text-[11px] uppercase tracking-[0.16em] text-muted-foreground/80">
                      {artifact.openMode === "preview" ? "Apple Preview" : artifact.kind === "image" ? "Inline" : "System App"}
                    </span>
                  </div>
                  <p className="mt-1 truncate text-sm font-medium text-foreground">
                    {artifact.filename}
                  </p>
                  <p className="truncate text-xs text-muted-foreground">
                    {artifact.path}
                  </p>
                </div>

                {artifact.kind !== "image" && (
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={openingPath === artifact.path}
                    onClick={async () => {
                      setOpeningPath(artifact.path);
                      setErrorByPath((prev) => ({ ...prev, [artifact.path]: "" }));
                      try {
                        await tauri.openChatArtifact(artifact.path);
                      } catch (error) {
                        setErrorByPath((prev) => ({
                          ...prev,
                          [artifact.path]:
                            error instanceof Error ? error.message : String(error),
                        }));
                      } finally {
                        setOpeningPath((current) =>
                          current === artifact.path ? null : current,
                        );
                      }
                    }}
                  >
                    {artifact.openMode === "preview" ? (
                      <Eye className="size-3.5" />
                    ) : (
                      <ExternalLink className="size-3.5" />
                    )}
                    {actionLabel(artifact.openMode)}
                  </Button>
                )}
              </CardContent>
            </Card>

            {errorByPath[artifact.path] ? (
              <p className="px-1 text-xs text-red-500">{errorByPath[artifact.path]}</p>
            ) : null}
          </div>
        );
      })}
    </div>
  );
});
