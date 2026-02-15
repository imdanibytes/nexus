import { useCallback, useEffect, useState } from "react";
import type { RegistryKind, RegistrySource } from "../../types/plugin";
import * as api from "../../lib/tauri";
import { Database, FolderOpen, Globe, Plus, Trash2 } from "lucide-react";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

const PROTECTED_REGISTRIES = new Set(["nexus-community", "nexus-mcp-local"]);

export function RegistrySettings() {
  const [registries, setRegistries] = useState<RegistrySource[]>([]);
  const [showAdd, setShowAdd] = useState(false);
  const [newName, setNewName] = useState("");
  const [newKind, setNewKind] = useState<RegistryKind>("local");
  const [newUrl, setNewUrl] = useState("");
  const [adding, setAdding] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const list = await api.registryList();
      setRegistries(list);
    } catch {
      // silently fail
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  async function handleAdd() {
    if (!newName.trim() || !newUrl.trim()) return;
    setAdding(true);
    try {
      await api.registryAdd(newName.trim(), newKind, newUrl.trim());
      setNewName("");
      setNewUrl("");
      setShowAdd(false);
      await refresh();
    } catch {
      // TODO: show error
    } finally {
      setAdding(false);
    }
  }

  async function handleRemove(id: string) {
    try {
      await api.registryRemove(id);
      await refresh();
    } catch {
      // TODO: show error
    }
  }

  async function handleToggle(id: string, enabled: boolean) {
    try {
      await api.registryToggle(id, enabled);
      await refresh();
    } catch {
      // TODO: show error
    }
  }

  return (
    <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <Database size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <div>
            <h3 className="text-[14px] font-semibold text-nx-text">Registries</h3>
            <p className="text-[11px] text-nx-text-ghost mt-0.5">
              Plugin sources for the marketplace
            </p>
          </div>
        </div>
        <Button
          size="sm"
          variant={showAdd ? "secondary" : "default"}
          onClick={() => setShowAdd(!showAdd)}
        >
          <Plus size={12} strokeWidth={1.5} />
          {showAdd ? "Cancel" : "Add Registry"}
        </Button>
      </div>

      {/* Add form */}
      {showAdd && (
        <div className="mb-4 p-4 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle space-y-3">
          <div>
            <label className="block text-[11px] font-medium text-nx-text-muted mb-1.5">
              Name
            </label>
            <Input
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder="My Private Registry"
            />
          </div>
          <div>
            <label className="block text-[11px] font-medium text-nx-text-muted mb-1.5">
              Type
            </label>
            <div className="flex gap-2">
              <Button
                variant="secondary"
                size="sm"
                onClick={() => setNewKind("local")}
                className={newKind === "local" ? "bg-nx-accent text-nx-deep hover:bg-nx-accent-hover" : "text-nx-text-muted hover:text-nx-text-secondary"}
              >
                <FolderOpen size={12} strokeWidth={1.5} />
                Local Path
              </Button>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => setNewKind("remote")}
                className={newKind === "remote" ? "bg-nx-accent text-nx-deep hover:bg-nx-accent-hover" : "text-nx-text-muted hover:text-nx-text-secondary"}
              >
                <Globe size={12} strokeWidth={1.5} />
                Remote URL
              </Button>
            </div>
          </div>
          <div>
            <label className="block text-[11px] font-medium text-nx-text-muted mb-1.5">
              {newKind === "local" ? "Directory Path" : "Registry URL"}
            </label>
            <Input
              value={newUrl}
              onChange={(e) => setNewUrl(e.target.value)}
              placeholder={
                newKind === "local"
                  ? "/path/to/my-registry"
                  : "https://example.com/registry/index.json"
              }
              className="font-mono"
            />
            <p className="text-[11px] text-nx-text-ghost mt-1.5">
              {newKind === "local"
                ? "Path to a registry directory with plugins/ and extensions/ YAML files"
                : "URL to the raw index.json file of the registry"}
            </p>
          </div>
          <Button
            size="sm"
            onClick={handleAdd}
            disabled={adding || !newName.trim() || !newUrl.trim()}
          >
            {adding ? "Adding..." : "Add Registry"}
          </Button>
        </div>
      )}

      {/* Registry list */}
      <div className="space-y-2">
        {registries.length === 0 ? (
          <p className="text-[11px] text-nx-text-ghost">No registries configured</p>
        ) : (
          registries.map((reg) => (
            <div
              key={reg.id}
              className="flex items-center justify-between p-3 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle hover:border-nx-border transition-colors duration-150"
            >
              <div className="flex items-center gap-3 min-w-0">
                <Switch size="sm" checked={reg.enabled} onCheckedChange={(checked) => handleToggle(reg.id, checked)} />
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="text-[13px] text-nx-text font-medium truncate">
                      {reg.name}
                    </span>
                    <span
                      className={`text-[10px] px-1.5 py-0.5 rounded-[var(--radius-tag)] font-semibold tracking-wide ${
                        reg.kind === "local"
                          ? "bg-nx-highlight-muted text-nx-highlight"
                          : "bg-nx-info-muted text-nx-info"
                      }`}
                    >
                      {reg.kind === "local" ? "LOCAL" : "REMOTE"}
                    </span>
                  </div>
                  <p className="text-[11px] text-nx-text-ghost truncate font-mono mt-0.5">
                    {reg.url}
                  </p>
                </div>
              </div>
              {!PROTECTED_REGISTRIES.has(reg.id) && (
                <Button
                  variant="ghost"
                  size="icon-xs"
                  onClick={() => handleRemove(reg.id)}
                  className="text-nx-text-ghost hover:text-nx-error flex-shrink-0 ml-2"
                  title="Remove registry"
                >
                  <Trash2 size={14} strokeWidth={1.5} />
                </Button>
              )}
            </div>
          ))
        )}
      </div>
    </section>
  );
}
