import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { RegistryKind, RegistrySource } from "../../types/plugin";
import * as api from "../../lib/tauri";
import { Database, FolderOpen, Globe, Plus, Trash2 } from "lucide-react";
import { Switch, Button, Input, Card, CardBody, Chip } from "@heroui/react";

const PROTECTED_REGISTRIES = new Set(["nexus-community", "nexus-mcp-local"]);

export function RegistrySettings() {
  const { t } = useTranslation("settings");
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
    <Card><CardBody className="p-5">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <Database size={15} strokeWidth={1.5} className="text-default-500" />
          <div>
            <h3 className="text-[14px] font-semibold">{t("registries.title")}</h3>
            <p className="text-[11px] text-default-400 mt-0.5">
              {t("registries.subtitle")}
            </p>
          </div>
        </div>
        <Button
          onPress={() => setShowAdd(!showAdd)}
        >
          <Plus size={12} strokeWidth={1.5} />
          {showAdd ? t("common:action.cancel") : t("registries.addRegistry")}
        </Button>
      </div>

      {/* Add form */}
      {showAdd && (
        <div className="mb-4 p-4 rounded-[8px] bg-background border border-default-100 space-y-3">
          <div>
            <label className="block text-[11px] font-medium text-default-500 mb-1.5">
              {t("registries.name")}
            </label>
            <Input
              value={newName}
              onValueChange={setNewName}
              placeholder={t("registries.namePlaceholder")}
              variant="bordered"
            />
          </div>
          <div>
            <label className="block text-[11px] font-medium text-default-500 mb-1.5">
              {t("registries.type")}
            </label>
            <div className="flex gap-2">
              <Button
                onPress={() => setNewKind("local")}
              >
                <FolderOpen size={12} strokeWidth={1.5} />
                {t("registries.localPath")}
              </Button>
              <Button
                onPress={() => setNewKind("remote")}
              >
                <Globe size={12} strokeWidth={1.5} />
                {t("registries.remoteUrl")}
              </Button>
            </div>
          </div>
          <div>
            <label className="block text-[11px] font-medium text-default-500 mb-1.5">
              {newKind === "local" ? t("registries.directoryPath") : t("registries.registryUrl")}
            </label>
            <Input
              value={newUrl}
              onValueChange={setNewUrl}
              placeholder={
                newKind === "local"
                  ? t("registries.localPathPlaceholder")
                  : t("registries.remoteUrlPlaceholder")
              }
              variant="bordered"
            />
            <p className="text-[11px] text-default-400 mt-1.5">
              {newKind === "local"
                ? t("registries.localHint")
                : t("registries.remoteHint")}
            </p>
          </div>
          <Button
            onPress={handleAdd}
            isDisabled={adding || !newName.trim() || !newUrl.trim()}
          >
            {adding ? t("registries.adding") : t("registries.addRegistry")}
          </Button>
        </div>
      )}

      {/* Registry list */}
      <div className="space-y-2">
        {registries.length === 0 ? (
          <p className="text-[11px] text-default-400">{t("registries.noRegistries")}</p>
        ) : (
          registries.map((reg) => (
            <div
              key={reg.id}
              className="flex items-center justify-between p-3 rounded-[8px] bg-background border border-default-100 hover:border-divider transition-colors duration-150"
            >
              <div className="flex items-center gap-3 min-w-0">
                <Switch isSelected={reg.enabled} onValueChange={(checked) => handleToggle(reg.id, checked)} />
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="text-[13px] font-medium truncate">
                      {reg.name}
                    </span>
                    <Chip
                      size="sm"
                      variant="flat"
                    >
                      {reg.kind === "local" ? t("registries.local") : t("registries.remote")}
                    </Chip>
                  </div>
                  <p className="text-[11px] text-default-400 truncate font-mono mt-0.5">
                    {reg.url}
                  </p>
                </div>
              </div>
              {!PROTECTED_REGISTRIES.has(reg.id) && (
                <Button
                  isIconOnly
                  onPress={() => handleRemove(reg.id)}
                  color="danger"
                  className="flex-shrink-0 ml-2"
                  title={t("registries.removeRegistry")}
                >
                  <Trash2 size={14} strokeWidth={1.5} />
                </Button>
              )}
            </div>
          ))
        )}
      </div>
    </CardBody></Card>
  );
}
