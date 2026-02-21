import { useTranslation } from "react-i18next";
import { Button } from "@heroui/react";
import { Plus, Trash2 } from "lucide-react";
import { ResourceStatusBadge } from "./ResourceStatusBadge";
import type { ResourceTypeDef } from "../../../types/extension";

function formatRelativeTime(value: unknown): string {
  if (!value) return "—";
  try {
    const date = new Date(String(value));
    const diffMs = Date.now() - date.getTime();
    const diffSec = Math.floor(diffMs / 1000);
    if (diffSec < 60) return `${diffSec}s ago`;
    const diffMin = Math.floor(diffSec / 60);
    if (diffMin < 60) return `${diffMin}m ago`;
    const diffHr = Math.floor(diffMin / 60);
    if (diffHr < 24) return `${diffHr}h ago`;
    return `${Math.floor(diffHr / 24)}d ago`;
  } catch {
    return String(value);
  }
}

interface SchemaProperty {
  type?: string;
  "x-display"?: {
    variant?: string;
  };
}

function formatCell(value: unknown, prop: SchemaProperty | undefined): React.ReactNode {
  const display = prop?.["x-display"];
  if (display?.variant === "status-indicator") {
    return <ResourceStatusBadge value={String(value ?? "")} />;
  }
  if (display?.variant === "relative-time") {
    return (
      <span className="text-[11px] text-default-400">
        {formatRelativeTime(value)}
      </span>
    );
  }
  if (value === null || value === undefined) return <span className="text-default-400">—</span>;
  if (typeof value === "boolean") return value ? "Yes" : "No";
  if (typeof value === "object") return <span className="font-mono text-[10px]">{JSON.stringify(value)}</span>;
  return <span>{String(value)}</span>;
}

interface ResourceListViewProps {
  typeDef: ResourceTypeDef;
  items: Record<string, unknown>[];
  loading: boolean;
  onCreateClick: () => void;
  onRowClick: (item: Record<string, unknown>) => void;
  onDeleteClick: (item: Record<string, unknown>) => void;
}

export function ResourceListView({
  typeDef,
  items,
  loading,
  onCreateClick,
  onRowClick,
  onDeleteClick,
}: ResourceListViewProps) {
  const { t } = useTranslation("settings");
  const columns = typeDef.list_view?.columns ?? [];
  const schema = typeDef.schema as { properties?: Record<string, SchemaProperty> };
  const properties = schema.properties ?? {};

  const canCreate = typeDef.capabilities?.create !== false;
  const canDelete = typeDef.capabilities?.delete !== false;

  return (
    <div>
      <div className="flex items-center justify-between mb-3">
        <span className="text-[11px] text-default-400">
          {loading ? "Loading..." : t("extensionsTab.resourceEmpty", { label: items.length === 0 ? typeDef.label : "" })}
        </span>
        {canCreate && (
          <Button size="sm" onPress={onCreateClick}>
            <Plus size={11} strokeWidth={1.5} />
            {t("extensionsTab.resourceCreate", { label: typeDef.label })}
          </Button>
        )}
      </div>

      {items.length > 0 && (
        <div className="rounded-[8px] border border-default-100 overflow-hidden">
          <table className="w-full text-left">
            <thead>
              <tr className="border-b border-default-100 bg-default-50">
                {columns.map((col) => (
                  <th key={col} className="px-3 py-2 text-[10px] font-semibold text-default-500 uppercase tracking-wide">
                    {col}
                  </th>
                ))}
                {canDelete && (
                  <th className="px-3 py-2 text-[10px] font-semibold text-default-500 uppercase tracking-wide w-10" />
                )}
              </tr>
            </thead>
            <tbody>
              {items.map((item, idx) => (
                <tr
                  key={idx}
                  className="border-b border-default-100 last:border-0 hover:bg-default-100/40 cursor-pointer transition-colors"
                  // eslint-disable-next-line react-perf/jsx-no-new-function-as-prop
                  onClick={() => onRowClick(item)}
                >
                  {columns.map((col) => (
                    <td key={col} className="px-3 py-2 text-[12px]">
                      {formatCell(item[col], properties[col])}
                    </td>
                  ))}
                  {canDelete && (
                    // eslint-disable-next-line react-perf/jsx-no-new-function-as-prop
                    <td className="px-3 py-2" onClick={(e) => e.stopPropagation()}>
                      <Button
                        size="sm"
                        isIconOnly
                        variant="light"
                        color="danger"
                        // eslint-disable-next-line react-perf/jsx-no-new-function-as-prop
                        onPress={() => onDeleteClick(item)}
                        className="min-w-0 h-6 w-6"
                      >
                        <Trash2 size={11} strokeWidth={1.5} />
                      </Button>
                    </td>
                  )}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {items.length === 0 && !loading && (
        <p className="text-[11px] text-default-400 px-1 py-2">
          {t("extensionsTab.resourceEmpty", { label: typeDef.label })}
        </p>
      )}
    </div>
  );
}
