import { useState } from "react";
import { useAppStore } from "../../stores/appStore";
import { PermissionList } from "../permissions/PermissionList";
import { Search, ChevronDown, Shield } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Collapsible, CollapsibleTrigger, CollapsibleContent } from "@/components/ui/collapsible";

export function PermissionsTab() {
  const { installedPlugins } = useAppStore();
  const [search, setSearch] = useState("");
  const [expanded, setExpanded] = useState<Set<string>>(new Set());

  const filtered = installedPlugins.filter((p) =>
    p.manifest.name.toLowerCase().includes(search.toLowerCase()) ||
    p.manifest.id.toLowerCase().includes(search.toLowerCase())
  );

  function toggle(id: string) {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  }

  return (
    <div className="space-y-6">
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center gap-2 mb-4">
          <Shield size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <h3 className="text-[14px] font-semibold text-nx-text">
            Plugin Permissions
          </h3>
        </div>

        {installedPlugins.length === 0 ? (
          <p className="text-[11px] text-nx-text-ghost">
            No plugins installed
          </p>
        ) : (
          <>
            {/* Search */}
            <div className="relative mb-4">
              <Search
                size={14}
                strokeWidth={1.5}
                className="absolute left-3 top-1/2 -translate-y-1/2 text-nx-text-ghost"
              />
              <Input
                type="text"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                placeholder="Filter plugins..."
                className="pl-9"
              />
            </div>

            {/* Plugin accordion */}
            <div className="space-y-2">
              {filtered.length === 0 ? (
                <p className="text-[11px] text-nx-text-ghost">
                  No plugins match "{search}"
                </p>
              ) : (
                filtered.map((plugin) => {
                  const id = plugin.manifest.id;
                  const isOpen = expanded.has(id);
                  const permCount = plugin.manifest.permissions.length;

                  return (
                    <Collapsible key={id} open={isOpen} onOpenChange={() => toggle(id)}>
                      <div className="rounded-[var(--radius-button)] border border-nx-border-subtle bg-nx-deep overflow-hidden">
                        <CollapsibleTrigger asChild>
                          <button
                            className="w-full flex items-center justify-between p-3 hover:bg-nx-wash/30 transition-colors duration-150"
                          >
                            <div className="flex items-center gap-3 min-w-0">
                              <span className="text-[13px] text-nx-text font-medium truncate">
                                {plugin.manifest.name}
                              </span>
                              <span className="text-[11px] text-nx-text-ghost font-mono flex-shrink-0">
                                v{plugin.manifest.version}
                              </span>
                              <Badge variant={plugin.status === "running" ? "success" : "secondary"} className="text-[10px]">
                                {plugin.status}
                              </Badge>
                            </div>
                            <div className="flex items-center gap-2 flex-shrink-0 ml-2">
                              <span className="text-[11px] text-nx-text-ghost">
                                {permCount} perm{permCount !== 1 ? "s" : ""}
                              </span>
                              <ChevronDown
                                size={14}
                                strokeWidth={1.5}
                                className={`text-nx-text-ghost transition-transform duration-200 ${
                                  isOpen ? "rotate-180" : ""
                                }`}
                              />
                            </div>
                          </button>
                        </CollapsibleTrigger>
                        <CollapsibleContent>
                          <div className="px-3 pb-3 border-t border-nx-border-subtle">
                            <div className="pt-3">
                              <PermissionList pluginId={id} />
                            </div>
                          </div>
                        </CollapsibleContent>
                      </div>
                    </Collapsible>
                  );
                })
              )}
            </div>
          </>
        )}
      </section>
    </div>
  );
}
