import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Tabs, Tab } from "@heroui/react";
import { Database } from "lucide-react";
import type { ExtensionStatus } from "../../../types/extension";
import { useExtensionResources } from "./useExtensionResources";
import { ResourceListView } from "./ResourceListView";
import { ResourceForm } from "./ResourceForm";
import { ResourceDeleteDialog } from "./ResourceDeleteDialog";

interface ResourceTypeTabProps {
  extId: string;
  resourceType: string;
  typeDef: ExtensionStatus["resources"][string];
}

function ResourceTypeTab({ extId, resourceType, typeDef }: ResourceTypeTabProps) {
  const { items, loading, refresh, create, update, remove } = useExtensionResources(extId, resourceType);
  const [formOpen, setFormOpen] = useState(false);
  const [formMode, setFormMode] = useState<"create" | "edit">("create");
  const [editingItem, setEditingItem] = useState<Record<string, unknown> | undefined>();
  const [deleteTarget, setDeleteTarget] = useState<Record<string, unknown> | null>(null);

  useEffect(() => {
    refresh();
  }, [refresh]);

  function handleCreateClick() {
    setFormMode("create");
    setEditingItem(undefined);
    setFormOpen(true);
  }

  function handleRowClick(item: Record<string, unknown>) {
    if (typeDef.capabilities?.update === false) return;
    setFormMode("edit");
    setEditingItem(item);
    setFormOpen(true);
  }

  function handleDeleteClick(item: Record<string, unknown>) {
    setDeleteTarget(item);
  }

  async function handleFormSubmit(data: Record<string, unknown>) {
    if (formMode === "create") {
      await create(data);
    } else if (editingItem) {
      const id = String(editingItem.id ?? editingItem._id ?? "");
      await update(id, data);
    }
  }

  async function handleDeleteConfirm() {
    if (!deleteTarget) return;
    const id = String(deleteTarget.id ?? deleteTarget._id ?? "");
    await remove(id);
    setDeleteTarget(null);
  }

  return (
    <div>
      <ResourceListView
        typeDef={typeDef}
        items={items}
        loading={loading}
        onCreateClick={handleCreateClick}
        onRowClick={handleRowClick}
        onDeleteClick={handleDeleteClick}
      />
      <ResourceForm
        isOpen={formOpen}
        mode={formMode}
        typeDef={typeDef}
        initialValues={editingItem}
        onClose={() => setFormOpen(false)}
        onSubmit={handleFormSubmit}
      />
      <ResourceDeleteDialog
        isOpen={!!deleteTarget}
        label={typeDef.label}
        onClose={() => setDeleteTarget(null)}
        onConfirm={handleDeleteConfirm}
      />
    </div>
  );
}

interface ResourcePanelProps {
  extension: ExtensionStatus;
}

export function ResourcePanel({ extension }: ResourcePanelProps) {
  const { t } = useTranslation("settings");
  const resourceTypes = Object.entries(extension.resources ?? {});

  if (resourceTypes.length === 0) return null;

  return (
    <div>
      <div className="flex items-center gap-2 mb-3">
        <Database size={12} strokeWidth={1.5} className="text-default-400" />
        <span className="text-[11px] font-semibold text-default-500 uppercase tracking-wide">
          {t("extensionsTab.resources")}
        </span>
      </div>
      {resourceTypes.length === 1 ? (
        <ResourceTypeTab
          extId={extension.id}
          resourceType={resourceTypes[0][0]}
          typeDef={resourceTypes[0][1]}
        />
      ) : (
        <Tabs size="sm" variant="underlined" classNames={{ tab: "text-[11px]", tabContent: "text-[11px]" }}>
          {resourceTypes.map(([key, typeDef]) => (
            <Tab key={key} title={typeDef.label}>
              <ResourceTypeTab
                extId={extension.id}
                resourceType={key}
                typeDef={typeDef}
              />
            </Tab>
          ))}
        </Tabs>
      )}
    </div>
  );
}
