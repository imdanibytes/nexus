import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { auditQuery, auditCount, auditExport } from "../../lib/tauri";
import type { AuditLogRow } from "../../types/audit";
import {
  ScrollText,
  Download,
  ChevronDown,
  Check,
  X,
  User,
  Monitor,
  Puzzle,
  Bot,
} from "lucide-react";
import {
  Button,
  Card,
  CardBody,
  Chip,
  Pagination,
  Select,
  SelectItem,
  Input,
} from "@heroui/react";

const PAGE_SIZES = [25, 50, 100];

const ACTION_CATEGORIES = [
  { key: "all", label: "All" },
  { key: "plugin", label: "Plugin" },
  { key: "extension", label: "Extension" },
  { key: "permission", label: "Permission" },
  { key: "security", label: "Security" },
  { key: "settings", label: "Settings" },
  { key: "mcp", label: "MCP" },
] as const;

const SEVERITY_OPTIONS = [
  { key: "all", label: "All" },
  { key: "info", label: "Info" },
  { key: "warn", label: "Warn" },
  { key: "critical", label: "Critical" },
] as const;

type SeverityFilter = (typeof SEVERITY_OPTIONS)[number]["key"];

type ActionCategory = (typeof ACTION_CATEGORIES)[number]["key"];

function actionCategoryToGlob(cat: ActionCategory): string | undefined {
  if (cat === "all") return undefined;
  if (cat === "security") return "security.*";
  return `${cat}.*`;
}

function actionColor(
  action: string
): "primary" | "success" | "warning" | "danger" | "secondary" | "default" {
  if (action.startsWith("plugin.")) return "primary";
  if (action.startsWith("extension.")) return "secondary";
  if (action.startsWith("permission.")) return "warning";
  if (action.startsWith("security.")) return "danger";
  if (action.startsWith("settings.")) return "default";
  if (action.startsWith("mcp.")) return "success";
  return "default";
}

function severityColor(severity: string): "default" | "warning" | "danger" {
  if (severity === "critical") return "danger";
  if (severity === "warn") return "warning";
  return "default";
}

function ActorIcon({ actor }: { actor: string }) {
  if (actor === "system")
    return <Monitor size={12} strokeWidth={1.5} className="text-default-400" />;
  if (actor === "mcp_client")
    return <Bot size={12} strokeWidth={1.5} className="text-success" />;
  if (actor.startsWith("plugin:"))
    return <Puzzle size={12} strokeWidth={1.5} className="text-primary" />;
  return <User size={12} strokeWidth={1.5} className="text-default-500" />;
}

function ResultIcon({ result }: { result: string }) {
  if (result === "success")
    return <Check size={12} strokeWidth={2} className="text-success" />;
  return <X size={12} strokeWidth={2} className="text-danger" />;
}

function formatTime(ts: string): string {
  try {
    const d = new Date(ts);
    return d.toLocaleString(undefined, {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  } catch {
    return ts;
  }
}

function DetailsView({ details }: { details: Record<string, unknown> }) {
  return (
    <div className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-[11px]">
      {Object.entries(details).map(([key, value]) => (
        <div key={key} className="contents">
          <span className="text-default-400 font-medium">{key}</span>
          <span className="text-default-600 font-mono break-all">
            {typeof value === "string"
              ? value
              : JSON.stringify(value)}
          </span>
        </div>
      ))}
    </div>
  );
}

export function AuditTab() {
  const { t } = useTranslation("settings");
  const [rows, setRows] = useState<AuditLogRow[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(25);
  const [category, _setCategory] = useState<ActionCategory>("all");
  const [resultFilter, _setResultFilter] = useState<"all" | "success" | "failure">("all");
  const [severityFilter, _setSeverityFilter] = useState<SeverityFilter>("all");
  const [subjectFilter, _setSubjectFilter] = useState("");
  const [expanded, setExpanded] = useState<Set<number>>(new Set());

  const actionGlob = useMemo(() => actionCategoryToGlob(category), [category]);

  const queryParams = useMemo(
    () => ({
      action: actionGlob,
      result: resultFilter === "all" ? undefined : resultFilter,
      severity: severityFilter === "all" ? undefined : severityFilter,
      subject: subjectFilter.trim() || undefined,
    }),
    [actionGlob, resultFilter, severityFilter, subjectFilter]
  );

  const load = useCallback(() => {
    auditQuery({
      ...queryParams,
      limit: pageSize,
      offset: (page - 1) * pageSize,
    })
      .then(setRows)
      .catch(() => {});
    auditCount(queryParams)
      .then(setTotal)
      .catch(() => {});
  }, [queryParams, page, pageSize]);

  // Wrap filter setters to also reset pagination
  const setCategory = useCallback((v: ActionCategory) => { _setCategory(v); setPage(1); }, []);
  const setResultFilter = useCallback((v: "all" | "success" | "failure") => { _setResultFilter(v); setPage(1); }, []);
  const setSeverityFilter = useCallback((v: SeverityFilter) => { _setSeverityFilter(v); setPage(1); }, []);
  const setSubjectFilter = useCallback((v: string) => { _setSubjectFilter(v); setPage(1); }, []);

  useEffect(() => {
    load();
    const id = setInterval(load, 5000);
    return () => clearInterval(id);
  }, [load]);

  const totalPages = Math.max(1, Math.ceil(total / pageSize));

  function toggleExpand(id: number) {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  async function handleExport() {
    try {
      const json = await auditExport({});
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `nexus-audit-${new Date().toISOString().slice(0, 10)}.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch {
      /* ignore */
    }
  }

  return (
    <div className="space-y-4">
      <Card>
        <CardBody className="p-5">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-2">
              <ScrollText size={15} strokeWidth={1.5} className="text-default-500" />
              <h3 className="text-[14px] font-semibold">{t("auditTab.title")}</h3>
            </div>
            <Button
              size="sm"
              variant="flat"
              startContent={<Download size={12} strokeWidth={1.5} />}
              onPress={handleExport}
            >
              {t("auditTab.export")}
            </Button>
          </div>

          <p className="text-[11px] text-default-400 mb-4">
            {t("auditTab.description")}
          </p>

          {/* Filters */}
          <div className="flex items-center gap-3 mb-4 flex-wrap">
            <Select
              size="sm"
              label={t("auditTab.category")}
              selectedKeys={[category]}
              onSelectionChange={(keys) => {
                const val = Array.from(keys)[0] as ActionCategory;
                if (val) setCategory(val);
              }}
              className="w-40"
            >
              {ACTION_CATEGORIES.map((c) => (
                <SelectItem key={c.key}>{c.label}</SelectItem>
              ))}
            </Select>

            <div className="flex gap-1">
              {(["all", "success", "failure"] as const).map((r) => (
                <Chip
                  key={r}
                  variant={resultFilter === r ? "solid" : "flat"}
                  color={r === "success" ? "success" : r === "failure" ? "danger" : "default"}
                  className="cursor-pointer"
                  onClick={() => setResultFilter(r)}
                >
                  {t(`auditTab.result_${r}`)}
                </Chip>
              ))}
            </div>

            <div className="flex gap-1">
              {SEVERITY_OPTIONS.map((s) => (
                <Chip
                  key={s.key}
                  variant={severityFilter === s.key ? "solid" : "flat"}
                  color={s.key === "critical" ? "danger" : s.key === "warn" ? "warning" : "default"}
                  className="cursor-pointer"
                  onClick={() => setSeverityFilter(s.key)}
                >
                  {t(`auditTab.severity_${s.key}`)}
                </Chip>
              ))}
            </div>

            <Input
              size="sm"
              variant="bordered"
              placeholder={t("auditTab.filterSubject")}
              value={subjectFilter}
              onValueChange={setSubjectFilter}
              className="w-48"
            />
          </div>

          {/* Table */}
          {rows.length === 0 ? (
            <p className="text-[11px] text-default-400 py-4 text-center">
              {t("auditTab.noEntries")}
            </p>
          ) : (
            <div className="space-y-1">
              {rows.map((row) => {
                const isOpen = expanded.has(row.id);
                return (
                  <Card key={row.id} shadow="none" className="border border-default-200">
                    <CardBody
                      as="button"
                      onClick={() => toggleExpand(row.id)}
                      className="p-3 flex-row items-center gap-3 cursor-pointer"
                    >
                      <span className="text-[10px] text-default-400 font-mono w-32 flex-shrink-0 text-left">
                        {formatTime(row.timestamp)}
                      </span>
                      <Chip size="sm" variant="flat" color={actionColor(row.action)}>
                        {row.action}
                      </Chip>
                      <Chip size="sm" variant="dot" color={severityColor(row.severity)}>
                        {row.severity}
                      </Chip>
                      <div className="flex items-center gap-1 flex-shrink-0">
                        <ActorIcon actor={row.actor} />
                        <span className="text-[11px] text-default-500">{row.actor}</span>
                      </div>
                      {row.subject && (
                        <span className="text-[11px] text-default-600 font-mono truncate">
                          {row.subject}
                        </span>
                      )}
                      <div className="ml-auto flex items-center gap-2 flex-shrink-0">
                        <ResultIcon result={row.result} />
                        {row.details && (
                          <ChevronDown
                            size={14}
                            strokeWidth={1.5}
                            className={`text-default-400 transition-transform duration-200 ${
                              isOpen ? "rotate-180" : ""
                            }`}
                          />
                        )}
                      </div>
                    </CardBody>
                    {isOpen && row.details && (
                      <CardBody className="px-3 pb-3 pt-0 border-t border-default-100">
                        <DetailsView details={row.details} />
                      </CardBody>
                    )}
                  </Card>
                );
              })}
            </div>
          )}

          {/* Pagination */}
          {total > 0 && (
            <div className="flex items-center justify-between mt-4">
              <div className="flex items-center gap-2">
                <span className="text-[11px] text-default-400">
                  {t("auditTab.totalEntries", { count: total })}
                </span>
                <Select
                  size="sm"
                  selectedKeys={[String(pageSize)]}
                  onSelectionChange={(keys) => {
                    const val = Number(Array.from(keys)[0]);
                    if (val) {
                      setPageSize(val);
                      setPage(1);
                    }
                  }}
                  className="w-20"
                >
                  {PAGE_SIZES.map((s) => (
                    <SelectItem key={String(s)}>{String(s)}</SelectItem>
                  ))}
                </Select>
              </div>
              {totalPages > 1 && (
                <Pagination
                  size="sm"
                  total={totalPages}
                  page={page}
                  onChange={setPage}
                />
              )}
            </div>
          )}
        </CardBody>
      </Card>
    </div>
  );
}
