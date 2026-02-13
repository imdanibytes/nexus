import { useCallback, useEffect, useState } from "react";
import { extensionList } from "../../lib/tauri";
import type { ExtensionStatus } from "../../types/extension";
import {
  Blocks,
  ChevronDown,
  Shield,
  ShieldAlert,
  Puzzle,
} from "lucide-react";

const RISK_STYLES: Record<string, { bg: string; text: string }> = {
  low: { bg: "bg-nx-success-muted", text: "text-nx-success" },
  medium: { bg: "bg-nx-warning-muted", text: "text-nx-warning" },
  high: { bg: "bg-nx-error-muted", text: "text-nx-error" },
};

export function ExtensionsTab() {
  const [extensions, setExtensions] = useState<ExtensionStatus[]>([]);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const exts = await extensionList();
      setExtensions(exts);
    } catch {
      // backend may not have extension commands yet
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

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

  if (loading) {
    return (
      <div className="space-y-6">
        <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
          <p className="text-[12px] text-nx-text-ghost">
            Loading extensions...
          </p>
        </section>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center gap-2 mb-2">
          <Blocks size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <h3 className="text-[14px] font-semibold text-nx-text">
            Host Extensions
          </h3>
        </div>
        <p className="text-[11px] text-nx-text-ghost">
          Extensions run trusted code on the host and expose typed, validated
          operations to plugins. Plugins consume extensions through the Host API
          to perform privileged tasks like credential management and cache
          control.
        </p>
        <div className="mt-3 flex items-center gap-2">
          <span className="text-[11px] text-nx-text-muted font-medium">
            {extensions.length} extension{extensions.length !== 1 ? "s" : ""}{" "}
            registered
          </span>
        </div>
      </section>

      {/* Extension cards */}
      {extensions.length === 0 ? (
        <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
          <p className="text-[12px] text-nx-text-ghost">
            No extensions registered.
          </p>
        </section>
      ) : (
        extensions.map((ext) => {
          const isOpen = expanded.has(ext.id);
          return (
            <section
              key={ext.id}
              className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border overflow-hidden"
            >
              {/* Extension header */}
              <button
                onClick={() => toggleExpanded(ext.id)}
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
                  </div>
                  <p className="text-[11px] text-nx-text-ghost">
                    {ext.description}
                  </p>
                  <div className="flex items-center gap-3 mt-2">
                    <span className="text-[10px] text-nx-text-muted">
                      {ext.operations.length} operation
                      {ext.operations.length !== 1 ? "s" : ""}
                    </span>
                    {ext.consumers.length > 0 && (
                      <span className="text-[10px] text-nx-text-muted">
                        {ext.consumers.length} plugin
                        {ext.consumers.length !== 1 ? "s" : ""}
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

              {/* Expanded detail */}
              {isOpen && (
                <div className="border-t border-nx-border">
                  {/* Operations */}
                  <div className="p-4">
                    <div className="flex items-center gap-2 mb-3">
                      <Blocks
                        size={12}
                        strokeWidth={1.5}
                        className="text-nx-text-ghost"
                      />
                      <span className="text-[11px] font-semibold text-nx-text-muted uppercase tracking-wide">
                        Operations
                      </span>
                    </div>
                    <div className="space-y-1">
                      {ext.operations.map((op) => {
                        const risk =
                          RISK_STYLES[op.risk_level] ?? RISK_STYLES.medium;
                        return (
                          <div
                            key={op.name}
                            className="flex items-center gap-3 px-3 py-2 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle"
                          >
                            <span className="text-[12px] text-nx-text font-mono min-w-0 flex-shrink-0">
                              {op.name}
                            </span>
                            <span
                              className={`text-[9px] font-semibold px-1.5 py-0.5 rounded-[var(--radius-tag)] flex-shrink-0 ${risk.bg} ${risk.text}`}
                            >
                              {op.risk_level}
                            </span>
                            <span className="text-[11px] text-nx-text-ghost truncate min-w-0 flex-1">
                              {op.description}
                            </span>
                          </div>
                        );
                      })}
                    </div>
                  </div>

                  {/* Plugin consumers */}
                  <div className="px-4 pb-4">
                    <div className="flex items-center gap-2 mb-3">
                      <Puzzle
                        size={12}
                        strokeWidth={1.5}
                        className="text-nx-text-ghost"
                      />
                      <span className="text-[11px] font-semibold text-nx-text-muted uppercase tracking-wide">
                        Plugin Consumers
                      </span>
                    </div>
                    {ext.consumers.length === 0 ? (
                      <p className="text-[11px] text-nx-text-ghost px-3">
                        No plugins using this extension.
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
                            <span className="relative group flex-shrink-0">
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
                              <span className="absolute bottom-full left-1/2 -translate-x-1/2 mb-1.5 px-2 py-1 text-[10px] font-medium text-nx-text bg-nx-surface border border-nx-border rounded-[var(--radius-tag)] shadow-sm whitespace-nowrap opacity-0 pointer-events-none group-hover:opacity-100 transition-opacity duration-150 z-10">
                                {consumer.granted
                                  ? "All extension permissions granted"
                                  : "Some extension permissions missing"}
                              </span>
                            </span>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                </div>
              )}
            </section>
          );
        })
      )}
    </div>
  );
}
