// src/components/agent-chat/PlanConfirmationCard.tsx
import React from "react";
import { Card } from "../ui/card";
import { Button } from "../ui/button";
import { Badge } from "../ui/badge";
import { FileCode, Play, FileText, FolderOpen, Search } from "lucide-react";
import { jsxDEV } from "react/jsx-dev-runtime";
var methodIcons = {
  write_file: FileCode,
  append_file: FileCode,
  read_file: FileText,
  list_files: FolderOpen,
  search_files: Search,
  default: FileCode
};
var methodColors = {
  write_file: "text-purple-400 bg-purple-400/10 border-purple-400/20",
  append_file: "text-green-400 bg-green-400/10 border-green-400/20",
  read_file: "text-blue-400 bg-blue-400/10 border-blue-400/20",
  list_files: "text-yellow-400 bg-yellow-400/10 border-yellow-400/20",
  search_files: "text-pink-400 bg-pink-400/10 border-pink-400/20",
  default: "text-gray-400 bg-gray-400/10 border-gray-400/20"
};
var PlanConfirmationCard = React.memo(function PlanConfirmationCard2({
  toolCalls,
  onExecute,
  isExecuting
}) {
  if (!toolCalls || toolCalls.length === 0)
    return null;
  return /* @__PURE__ */ jsxDEV(Card, {
    className: "w-full max-w-md p-4 space-y-4 border-l-4 border-l-purple-500 bg-purple-50/50 dark:bg-purple-900/10 mt-4",
    children: [
      /* @__PURE__ */ jsxDEV("div", {
        className: "flex items-center justify-between",
        children: [
          /* @__PURE__ */ jsxDEV("h3", {
            className: "font-medium text-sm flex items-center gap-2",
            children: [
              /* @__PURE__ */ jsxDEV(Play, {
                className: "size-4 text-purple-500"
              }, undefined, false, undefined, this),
              "Proposed Actions"
            ]
          }, undefined, true, undefined, this),
          /* @__PURE__ */ jsxDEV(Badge, {
            variant: "secondary",
            className: "bg-amber-500/10 text-amber-600 dark:text-amber-400 hover:bg-amber-500/20",
            children: [
              toolCalls.length,
              " operation",
              toolCalls.length !== 1 ? "s" : ""
            ]
          }, undefined, true, undefined, this)
        ]
      }, undefined, true, undefined, this),
      /* @__PURE__ */ jsxDEV("div", {
        className: "space-y-2 max-h-60 overflow-y-auto pr-1",
        children: toolCalls.map((call, idx) => {
          const Icon = methodIcons[call.method] || methodIcons.default;
          const colorClass = methodColors[call.method] || methodColors.default;
          return /* @__PURE__ */ jsxDEV("div", {
            className: `flex gap-3 items-start text-xs p-2.5 rounded-lg border ${colorClass} transition-all`,
            children: [
              /* @__PURE__ */ jsxDEV(Icon, {
                className: "size-4 mt-0.5 shrink-0"
              }, undefined, false, undefined, this),
              /* @__PURE__ */ jsxDEV("div", {
                className: "flex flex-col gap-0.5 overflow-hidden",
                children: [
                  /* @__PURE__ */ jsxDEV("span", {
                    className: "font-semibold font-mono text-[11px] uppercase opacity-70",
                    children: call.method.replace("_", " ")
                  }, undefined, false, undefined, this),
                  /* @__PURE__ */ jsxDEV("span", {
                    className: "truncate font-mono",
                    title: call.params.path,
                    children: call.params.path || call.params.query || "unknown"
                  }, undefined, false, undefined, this)
                ]
              }, undefined, true, undefined, this)
            ]
          }, idx, true, undefined, this);
        })
      }, undefined, false, undefined, this),
      /* @__PURE__ */ jsxDEV("div", {
        className: "flex gap-2 pt-2",
        children: /* @__PURE__ */ jsxDEV(Button, {
          className: "flex-1 bg-purple-600 hover:bg-purple-700 text-white shadow-lg shadow-purple-500/20",
          size: "sm",
          onClick: onExecute,
          disabled: isExecuting,
          children: [
            /* @__PURE__ */ jsxDEV(Play, {
              className: "size-3.5 fill-current"
            }, undefined, false, undefined, this),
            "Execute Plan"
          ]
        }, undefined, true, undefined, this)
      }, undefined, false, undefined, this)
    ]
  }, undefined, true, undefined, this);
});
export {
  PlanConfirmationCard
};
