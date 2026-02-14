import { useCallback, useEffect, useState } from "react";
import type { RegistryKind, RegistrySource } from "../../types/plugin";
import * as api from "../../lib/tauri";
import { Database, FolderOpen, Globe, Plus, Trash2 } from "lucide-react";

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
        <button
          onClick={() => setShowAdd(!showAdd)}
          className={`flex items-center gap-1.5 px-2.5 py-1 text-[11px] font-medium rounded-[var(--radius-button)] transition-all duration-150 ${
            showAdd
              ? "bg-nx-overlay text-nx-text-secondary hover:bg-nx-wash"
              : "bg-nx-accent hover:bg-nx-accent-hover text-nx-deep"
          }`}
        >
          <Plus size={12} strokeWidth={1.5} />
          {showAdd ? "Cancel" : "Add Registry"}
        </button>
      </div>

      {/* Add form */}
      {showAdd && (
        <div className="mb-4 p-4 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle space-y-3">
          <div>
            <label className="block text-[11px] font-medium text-nx-text-muted mb-1.5">
              Name
            </label>
            <input
              type="text"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder="My Private Registry"
              className="w-full px-3 py-2 text-[13px] bg-nx-wash border border-nx-border-strong rounded-[var(--radius-input)] text-nx-text placeholder:text-nx-text-muted focus:outline-none focus:shadow-[var(--shadow-focus)] transition-shadow duration-150"
            />
          </div>
          <div>
            <label className="block text-[11px] font-medium text-nx-text-muted mb-1.5">
              Type
            </label>
            <div className="flex gap-2">
              <button
                onClick={() => setNewKind("local")}
                className={`flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] transition-all duration-150 ${
                  newKind === "local"
                    ? "bg-nx-accent text-nx-deep"
                    : "bg-nx-overlay text-nx-text-muted hover:text-nx-text-secondary"
                }`}
              >
                <FolderOpen size={12} strokeWidth={1.5} />
                Local Path
              </button>
              <button
                onClick={() => setNewKind("remote")}
                className={`flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] transition-all duration-150 ${
                  newKind === "remote"
                    ? "bg-nx-accent text-nx-deep"
                    : "bg-nx-overlay text-nx-text-muted hover:text-nx-text-secondary"
                }`}
              >
                <Globe size={12} strokeWidth={1.5} />
                Remote URL
              </button>
            </div>
          </div>
          <div>
            <label className="block text-[11px] font-medium text-nx-text-muted mb-1.5">
              {newKind === "local" ? "Directory Path" : "Registry URL"}
            </label>
            <input
              type="text"
              value={newUrl}
              onChange={(e) => setNewUrl(e.target.value)}
              placeholder={
                newKind === "local"
                  ? "/path/to/my-registry"
                  : "https://example.com/registry/index.json"
              }
              className="w-full px-3 py-2 text-[13px] bg-nx-wash border border-nx-border-strong rounded-[var(--radius-input)] text-nx-text placeholder:text-nx-text-muted focus:outline-none focus:shadow-[var(--shadow-focus)] transition-shadow duration-150 font-mono"
            />
            <p className="text-[11px] text-nx-text-ghost mt-1.5">
              {newKind === "local"
                ? "Path to a registry directory with plugins/ and extensions/ YAML files"
                : "URL to the raw index.json file of the registry"}
            </p>
          </div>
          <button
            onClick={handleAdd}
            disabled={adding || !newName.trim() || !newUrl.trim()}
            className="px-3 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-accent hover:bg-nx-accent-hover disabled:opacity-40 text-nx-deep transition-all duration-150"
          >
            {adding ? "Adding..." : "Add Registry"}
          </button>
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
                <button
                  onClick={() => handleToggle(reg.id, !reg.enabled)}
                  className={`w-8 h-[18px] rounded-full relative transition-colors duration-150 flex-shrink-0 ${
                    reg.enabled ? "bg-nx-accent" : "bg-nx-wash"
                  }`}
                >
                  <span
                    className={`absolute top-[3px] w-3 h-3 rounded-full bg-white transition-all duration-150 ${
                      reg.enabled ? "left-[14px]" : "left-[3px]"
                    }`}
                  />
                </button>
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
              <button
                onClick={() => handleRemove(reg.id)}
                className="text-nx-text-ghost hover:text-nx-error transition-colors duration-150 flex-shrink-0 ml-2"
                title="Remove registry"
              >
                <Trash2 size={14} strokeWidth={1.5} />
              </button>
            </div>
          ))
        )}
      </div>
    </section>
  );
}
