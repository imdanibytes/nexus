import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useExtensions } from "../../hooks/useExtensions";
import { useAppStore } from "../../stores/appStore";
import {
  Blocks,
  ChevronDown,
  Shield,
  ShieldAlert,
  Puzzle,
  Plus,
  Power,
  Trash2,
  Loader2,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Tooltip, TooltipTrigger, TooltipContent } from "@/components/ui/tooltip";
import { Collapsible, CollapsibleTrigger, CollapsibleContent } from "@/components/ui/collapsible";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";

const RISK_VARIANT: Record<string, "success" | "warning" | "error"> = {
  low: "success",
  medium: "warning",
  high: "error",
};

export function ExtensionsTab() {
  const { t } = useTranslation("settings");
  const { extensions, busyExtensions, enable, disable, remove } = useExtensions();
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const { setView, focusExtensionId, setFocusExtensionId } = useAppStore();
  const [prevFocusId, setPrevFocusId] = useState<string | null>(null);

  // Adjust state during render: auto-expand the deep-linked extension
  if (focusExtensionId && focusExtensionId !== prevFocusId) {
    setPrevFocusId(focusExtensionId);
    setExpanded((prev) => new Set(prev).add(focusExtensionId));
    setFocusExtensionId(null);
  }

  function toggleExpanded(extId: string) {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(extId)) {
        next.delete(extId);
      } else {
        next.add(extId);
      }
      return next;
    });
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-start justify-between">
          <div>
            <div className="flex items-center gap-2 mb-2">
              <Blocks size={15} strokeWidth={1.5} className="text-nx-text-muted" />
              <h3 className="text-[14px] font-semibold text-nx-text">
                {t("extensionsTab.hostExtensions")}
              </h3>
            </div>
            <p className="text-[11px] text-nx-text-ghost">
              {t("extensionsTab.extensionsDesc")}
            </p>
            <div className="mt-3 flex items-center gap-2">
              <span className="text-[11px] text-nx-text-muted font-medium">
                {t("extensionsTab.extensionsCount", { count: extensions.length })}
              </span>
            </div>
          </div>
          <Button
            onClick={() => setView("extension-marketplace")}
            size="sm"
            className="flex-shrink-0 ml-4"
          >
            <Plus size={12} strokeWidth={1.5} />
            {t("extensionsTab.addExtension")}
          </Button>
        </div>
      </section>

      {/* Extension cards */}
      {extensions.length === 0 ? (
        <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
          <p className="text-[12px] text-nx-text-ghost">
            {t("extensionsTab.noExtensions")}
          </p>
        </section>
      ) : (
        extensions.map((ext) => {
          const isOpen = expanded.has(ext.id);
          const isBusy = !!busyExtensions[ext.id];
          return (
            <Collapsible
              key={ext.id}
              open={isOpen}
              onOpenChange={() => toggleExpanded(ext.id)}
            >
              <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border overflow-hidden">
                {/* Extension header */}
                <CollapsibleTrigger asChild>
                  <button
                    className="w-full flex items-center justify-between p-5 hover:bg-nx-wash/20 transition-colors"
                  >
                    <div className="min-w-0 flex-1 text-left">
                      <div className="flex items-center gap-2 mb-1">
                        <h4 className="text-[13px] font-semibold text-nx-text">
                          {ext.display_name}
                        </h4>
                        <span className="text-[10px] text-nx-text-ghost font-mono">
                          {ext.id}
                        </span>
                        {ext.installed && (
                          <Badge
                            variant={ext.enabled ? "success" : "secondary"}
                            className="text-[9px]"
                          >
                            {ext.enabled ? t("common:status.enabled") : t("common:status.disabled")}
                          </Badge>
                        )}
                      </div>
                      <p className="text-[11px] text-nx-text-ghost">
                        {ext.description}
                      </p>
                      <div className="flex items-center gap-3 mt-2">
                        <span className="text-[10px] text-nx-text-muted">
                          {t("extensionsTab.operationCount", { count: ext.operations.length })}
                        </span>
                        {ext.consumers.length > 0 && (
                          <span className="text-[10px] text-nx-text-muted">
                            {t("extensionsTab.pluginCount", { count: ext.consumers.length })}
                          </span>
                        )}
                      </div>
                    </div>
                    <ChevronDown
                      size={14}
                      strokeWidth={1.5}
                      className={`text-nx-text-ghost transition-transform duration-200 flex-shrink-0 ml-3 ${
                        isOpen ? "rotate-180" : ""
                      }`}
                    />
                  </button>
                </CollapsibleTrigger>

                {/* Expanded detail */}
                <CollapsibleContent>
                  <div className="border-t border-nx-border">
                    {/* Enable/Disable + Remove controls */}
                    {ext.installed && (
                      <div className="px-4 pt-4 flex items-center gap-2">
                        <Button
                          onClick={(e) => {
                            e.stopPropagation();
                            if (ext.enabled) disable(ext.id);
                            else enable(ext.id);
                          }}
                          disabled={isBusy}
                          variant={ext.enabled ? "secondary" : "default"}
                          size="xs"
                        >
                          {isBusy ? (
                            <Loader2 size={12} strokeWidth={1.5} className="animate-spin" />
                          ) : (
                            <Power size={12} strokeWidth={1.5} />
                          )}
                          {ext.enabled ? t("common:action.disable") : t("common:action.enable")}
                        </Button>
                        <Dialog>
                          <DialogTrigger asChild>
                            <Button
                              disabled={isBusy}
                              variant="destructive"
                              size="xs"
                              onClick={(e) => e.stopPropagation()}
                            >
                              <Trash2 size={12} strokeWidth={1.5} />
                              {t("common:action.remove")}
                            </Button>
                          </DialogTrigger>
                          <DialogContent className="sm:max-w-md" onClick={(e) => e.stopPropagation()}>
                            <DialogHeader>
                              <DialogTitle className="flex items-center gap-2 text-base">
                                {t("common:confirm.removeExtension", { name: ext.display_name })}
                              </DialogTitle>
                              <DialogDescription className="text-[13px] leading-relaxed pt-1" asChild>
                                <div>
                                  {ext.consumers.length > 0 ? (
                                    <>
                                      <p>
                                        {t("common:confirm.removeExtensionConsumers", { count: ext.consumers.length })}
                                      </p>
                                      <ul className="mt-2 space-y-1.5">
                                        {ext.consumers.map((c) => (
                                          <li
                                            key={c.plugin_id}
                                            className="flex items-center gap-2 px-3 py-2 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle"
                                          >
                                            <Puzzle size={12} strokeWidth={1.5} className="text-nx-text-ghost flex-shrink-0" />
                                            <span className="text-[12px] text-nx-text font-medium truncate">
                                              {c.plugin_name}
                                            </span>
                                            <span className="text-[10px] text-nx-text-ghost font-mono truncate ml-auto">
                                              {c.plugin_id}
                                            </span>
                                          </li>
                                        ))}
                                      </ul>
                                    </>
                                  ) : (
                                    <p>
                                      {t("common:confirm.removeExtensionNoConsumers")}
                                    </p>
                                  )}
                                </div>
                              </DialogDescription>
                            </DialogHeader>
                            <DialogFooter className="pt-2">
                              <DialogTrigger asChild>
                                <Button variant="secondary" size="sm">
                                  {t("common:action.cancel")}
                                </Button>
                              </DialogTrigger>
                              <DialogTrigger asChild>
                                <Button
                                  variant="destructive"
                                  size="sm"
                                  onClick={() => remove(ext.id)}
                                >
                                  {t("common:confirm.removeExtensionAction")}
                                </Button>
                              </DialogTrigger>
                            </DialogFooter>
                          </DialogContent>
                        </Dialog>
                      </div>
                    )}

                    {/* Operations */}
                    <div className="p-4">
                      <div className="flex items-center gap-2 mb-3">
                        <Blocks
                          size={12}
                          strokeWidth={1.5}
                          className="text-nx-text-ghost"
                        />
                        <span className="text-[11px] font-semibold text-nx-text-muted uppercase tracking-wide">
                          {t("extensionsTab.operations")}
                        </span>
                      </div>
                      <div className="space-y-1">
                        {ext.operations.map((op) => (
                          <div
                            key={op.name}
                            className="flex items-center gap-3 px-3 py-2 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle"
                          >
                            <span className="text-[12px] text-nx-text font-mono min-w-0 flex-shrink-0">
                              {op.name}
                            </span>
                            <Badge
                              variant={RISK_VARIANT[op.risk_level] ?? "warning"}
                              className="text-[9px]"
                            >
                              {op.risk_level}
                            </Badge>
                            {op.scope_key && (
                              <Badge variant="secondary" className="text-[9px] font-mono">
                                scope: {op.scope_key}
                              </Badge>
                            )}
                            <span className="text-[11px] text-nx-text-ghost truncate min-w-0 flex-1">
                              {op.description}
                            </span>
                          </div>
                        ))}
                      </div>
                    </div>

                    {/* Capabilities */}
                    {ext.capabilities.length > 0 && (
                      <div className="px-4 pb-4">
                        <div className="flex items-center gap-2 mb-3">
                          <Shield
                            size={12}
                            strokeWidth={1.5}
                            className="text-nx-text-ghost"
                          />
                          <span className="text-[11px] font-semibold text-nx-text-muted uppercase tracking-wide">
                            {t("extensionsTab.capabilities")}
                          </span>
                        </div>
                        <div className="flex gap-1.5 flex-wrap">
                          {ext.capabilities.map((cap, i) => (
                            <Badge key={i} variant="secondary">
                              {cap.type === "custom" ? cap.name : cap.type.replace(/_/g, " ")}
                            </Badge>
                          ))}
                        </div>
                      </div>
                    )}

                    {/* Plugin consumers */}
                    <div className="px-4 pb-4">
                      <div className="flex items-center gap-2 mb-3">
                        <Puzzle
                          size={12}
                          strokeWidth={1.5}
                          className="text-nx-text-ghost"
                        />
                        <span className="text-[11px] font-semibold text-nx-text-muted uppercase tracking-wide">
                          {t("extensionsTab.pluginConsumers")}
                        </span>
                      </div>
                      {ext.consumers.length === 0 ? (
                        <p className="text-[11px] text-nx-text-ghost px-3">
                          {t("extensionsTab.noConsumers")}
                        </p>
                      ) : (
                        <div className="space-y-1">
                          {ext.consumers.map((consumer) => (
                            <div
                              key={consumer.plugin_id}
                              className="flex items-center gap-3 px-3 py-2 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle"
                            >
                              <span className="text-[12px] text-nx-text font-medium truncate flex-1">
                                {consumer.plugin_name}
                              </span>
                              <Tooltip>
                                <TooltipTrigger asChild>
                                  <span className="flex-shrink-0">
                                    {consumer.granted ? (
                                      <Shield
                                        size={12}
                                        strokeWidth={1.5}
                                        className="text-nx-success cursor-help"
                                      />
                                    ) : (
                                      <ShieldAlert
                                        size={12}
                                        strokeWidth={1.5}
                                        className="text-nx-warning cursor-help"
                                      />
                                    )}
                                  </span>
                                </TooltipTrigger>
                                <TooltipContent>
                                  {consumer.granted
                                    ? t("extensionsTab.allPermsGranted")
                                    : t("extensionsTab.somePermsMissing")}
                                </TooltipContent>
                              </Tooltip>
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                  </div>
                </CollapsibleContent>
              </section>
            </Collapsible>
          );
        })
      )}
    </div>
  );
}
