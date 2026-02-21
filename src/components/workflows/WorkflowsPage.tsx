import { useState, useCallback, useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
  Button,
  Input,
  Switch,
  Select,
  SelectItem,
  Textarea,
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
  Tooltip,
  useDisclosure,
  Chip,
} from "@heroui/react";
import {
  Plus,
  Pencil,
  Trash2,
  Zap,
  ChevronDown,
  ChevronUp,
  Info,
  X,
} from "lucide-react";
import { Surface } from "@imdanibytes/nexus-ui";
import * as api from "../../lib/tauri";
import type { RoutingRule, Filter, RouteAction } from "../../types/workflows";
import type { McpToolStatus } from "../../types/mcp";
import { useAppStore } from "../../stores/appStore";

/* ─── Filter helpers ─── */

type FilterDialect = "exact" | "prefix" | "suffix";

interface SimpleFilter {
  dialect: FilterDialect;
  attribute: string;
  value: string;
}

function toSimpleFilters(filters: Filter[]): SimpleFilter[] {
  const result: SimpleFilter[] = [];
  for (const f of filters) {
    for (const dialect of ["exact", "prefix", "suffix"] as const) {
      const map = (f as Record<string, Record<string, string>>)[dialect];
      if (map) {
        for (const [attr, val] of Object.entries(map)) {
          result.push({ dialect, attribute: attr, value: val });
        }
      }
    }
  }
  return result.length > 0 ? result : [{ dialect: "exact", attribute: "type", value: "" }];
}

function fromSimpleFilters(simples: SimpleFilter[]): Filter[] {
  return simples
    .filter((s) => s.attribute && s.value)
    .map((s) => ({ [s.dialect]: { [s.attribute]: s.value } }) as Filter);
}

/* ─── Action helpers ─── */

function actionSummary(action: RouteAction, t: (key: string) => string): string {
  if (action.action === "invoke_plugin_tool") {
    return `${t("plugins:workflows.actionPluginTool")}: ${action.plugin_id} / ${action.tool_name}`;
  }
  if (action.action === "call_extension") {
    return `${t("plugins:workflows.actionExtensionOp")}: ${action.extension_id} / ${action.operation}`;
  }
  return `${t("plugins:workflows.actionFrontendEvent")}: ${action.channel}`;
}

function filterSummary(filters: Filter[], t: (key: string) => string): string {
  if (filters.length === 0) return t("plugins:workflows.matchesAll");
  const simple = toSimpleFilters(filters);
  return simple
    .map((s) => `${s.attribute} ${s.dialect} "${s.value}"`)
    .join(", ");
}

/* ─── Main page ─── */

export function WorkflowsPage() {
  const { t } = useTranslation(["plugins", "common"]);
  const [rules, setRules] = useState<RoutingRule[]>([]);
  const [loading, setLoading] = useState(true);
  const [editingRule, setEditingRule] = useState<RoutingRule | null>(null);
  const [isCreating, setIsCreating] = useState(false);

  const loadRules = useCallback(async () => {
    try {
      const data = await api.workflowList();
      setRules(data);
    } catch {
      // silent
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadRules();
  }, [loadRules]);

  const handleCreate = useCallback(() => {
    setEditingRule(null);
    setIsCreating(true);
  }, []);

  const handleEdit = useCallback((rule: RoutingRule) => {
    setEditingRule(rule);
    setIsCreating(true);
  }, []);

  const handleSaved = useCallback(() => {
    setIsCreating(false);
    setEditingRule(null);
    loadRules();
  }, [loadRules]);

  const handleCancel = useCallback(() => {
    setIsCreating(false);
    setEditingRule(null);
  }, []);

  const handleToggle = useCallback(
    async (rule: RoutingRule) => {
      try {
        await api.workflowUpdate({
          ruleId: rule.id,
          enabled: !rule.enabled,
        });
        loadRules();
      } catch {
        // silent
      }
    },
    [loadRules],
  );

  if (isCreating) {
    return (
      <WorkflowEditor
        rule={editingRule}
        onSave={handleSaved}
        onCancel={handleCancel}
      />
    );
  }

  return (
    <div className="max-w-4xl mx-auto p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">{t("plugins:workflows.title")}</h1>
          <p className="text-sm text-default-500 mt-1">
            {t("plugins:workflows.subtitle")}
          </p>
        </div>
        <Button color="primary" startContent={<Plus size={16} />} onPress={handleCreate}>
          {t("plugins:workflows.createWorkflow")}
        </Button>
      </div>

      {loading ? (
        <p className="text-default-400 text-sm">{t("common:action.loading")}</p>
      ) : rules.length === 0 ? (
        <EmptyState onCreate={handleCreate} />
      ) : (
        <div className="space-y-3">
          {rules.map((rule) => (
            <WorkflowCard
              key={rule.id}
              rule={rule}
              onEdit={handleEdit}
              onToggle={handleToggle}
              onDeleted={loadRules}
              t={t}
            />
          ))}
        </div>
      )}
    </div>
  );
}

/* ─── Empty state ─── */

function EmptyState({ onCreate }: { onCreate: () => void }) {
  const { t } = useTranslation(["plugins", "common"]);
  return (
    <Surface className="flex flex-col items-center justify-center py-16 px-8 text-center">
      <div className="w-16 h-16 rounded-2xl bg-default-100 flex items-center justify-center mb-4">
        <Zap size={28} strokeWidth={1.5} className="text-default-400" />
      </div>
      <h3 className="text-lg font-semibold text-default-600 mb-1">
        {t("plugins:workflows.noWorkflows")}
      </h3>
      <p className="text-sm text-default-500 max-w-md mb-6">
        {t("plugins:workflows.noWorkflowsHint")}
      </p>
      <Button color="primary" startContent={<Plus size={16} />} onPress={onCreate}>
        {t("plugins:workflows.createWorkflow")}
      </Button>
    </Surface>
  );
}

/* ─── Workflow card ─── */

function WorkflowCard({
  rule,
  onEdit,
  onToggle,
  onDeleted,
  t,
}: {
  rule: RoutingRule;
  onEdit: (rule: RoutingRule) => void;
  onToggle: (rule: RoutingRule) => void;
  onDeleted: () => void;
  t: (key: string) => string;
}) {
  const deleteModal = useDisclosure();

  const handleDelete = useCallback(async () => {
    deleteModal.onClose();
    try {
      await api.workflowDelete(rule.id);
      onDeleted();
    } catch {
      // silent
    }
  }, [rule.id, onDeleted, deleteModal]);

  const handleEdit = useCallback(() => onEdit(rule), [onEdit, rule]);
  const handleToggle = useCallback(() => onToggle(rule), [onToggle, rule]);

  return (
    <Surface className="p-4">
      <div className="flex items-start justify-between gap-4">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <Zap
              size={14}
              className={rule.enabled ? "text-primary" : "text-default-400"}
            />
            <span className="font-medium truncate">
              {rule.name || rule.id}
            </span>
            {!rule.enabled && (
              <Chip size="sm" variant="flat" color="default">
                {t("common:status.disabled")}
              </Chip>
            )}
          </div>
          <p className="text-xs text-default-500 truncate">
            {filterSummary(rule.filters, t)} &rarr; {actionSummary(rule.action, t)}
          </p>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <Switch
            size="sm"
            isSelected={rule.enabled}
            onValueChange={handleToggle}
          />
          <Tooltip content={t("plugins:workflows.editWorkflow")}>
            <Button
              isIconOnly
              size="sm"
              variant="light"
              onPress={handleEdit}
            >
              <Pencil size={14} />
            </Button>
          </Tooltip>
          <Tooltip content={t("plugins:workflows.deleteWorkflow")}>
            <Button
              isIconOnly
              size="sm"
              variant="light"
              color="danger"
              onPress={deleteModal.onOpen}
            >
              <Trash2 size={14} />
            </Button>
          </Tooltip>
        </div>
      </div>

      <Modal isOpen={deleteModal.isOpen} onOpenChange={deleteModal.onOpenChange}>
        <ModalContent>
          {(onClose) => (
            <>
              <ModalHeader>
                {t("plugins:workflows.deleteConfirm").replace(
                  "{{name}}",
                  rule.name || rule.id,
                )}
              </ModalHeader>
              <ModalBody>
                <p className="text-default-500">
                  {t("plugins:workflows.deleteConfirmDesc")}
                </p>
              </ModalBody>
              <ModalFooter>
                <Button variant="flat" onPress={onClose}>
                  {t("common:action.cancel")}
                </Button>
                <Button color="danger" onPress={handleDelete}>
                  {t("plugins:workflows.deleteAction")}
                </Button>
              </ModalFooter>
            </>
          )}
        </ModalContent>
      </Modal>
    </Surface>
  );
}

/* ─── Workflow editor ─── */

function WorkflowEditor({
  rule,
  onSave,
  onCancel,
}: {
  rule: RoutingRule | null;
  onSave: () => void;
  onCancel: () => void;
}) {
  const { t } = useTranslation(["plugins", "common"]);
  const isEdit = !!rule;

  // Form state
  const [name, setName] = useState(rule?.name ?? "");
  const [filters, setFilters] = useState<SimpleFilter[]>(
    rule ? toSimpleFilters(rule.filters) : [{ dialect: "exact", attribute: "type", value: "" }],
  );
  const [actionType, setActionType] = useState<RouteAction["action"]>(
    rule?.action.action ?? "invoke_plugin_tool",
  );
  const [pluginId, setPluginId] = useState(
    rule?.action.action === "invoke_plugin_tool" ? rule.action.plugin_id : "",
  );
  const [toolName, setToolName] = useState(
    rule?.action.action === "invoke_plugin_tool" ? rule.action.tool_name : "",
  );
  const [extensionId, setExtensionId] = useState(
    rule?.action.action === "call_extension" ? rule.action.extension_id : "",
  );
  const [operation, setOperation] = useState(
    rule?.action.action === "call_extension" ? rule.action.operation : "",
  );
  const [channel, setChannel] = useState(
    rule?.action.action === "emit_frontend" ? rule.action.channel : "",
  );
  const [argsTemplate, setArgsTemplate] = useState(() => {
    if (!rule) return "";
    const a = rule.action;
    if (a.action === "invoke_plugin_tool" || a.action === "call_extension") {
      return a.args_template ? JSON.stringify(a.args_template, null, 2) : "";
    }
    return "";
  });

  const [saving, setSaving] = useState(false);
  const [showTemplateHelp, setShowTemplateHelp] = useState(false);

  // Load available plugins/tools/extensions
  const [mcpTools, setMcpTools] = useState<McpToolStatus[]>([]);
  const installedExtensions = useAppStore((s) => s.installedExtensions);

  useEffect(() => {
    api.mcpListTools().then(setMcpTools).catch(() => {});
  }, []);

  // Derive plugin IDs from MCP tools
  const pluginIds = useMemo(() => {
    const ids = new Set<string>();
    for (const tool of mcpTools) {
      if (tool.plugin_id) ids.add(tool.plugin_id);
    }
    return Array.from(ids);
  }, [mcpTools]);

  // Tools for selected plugin
  const pluginTools = useMemo(
    () => mcpTools.filter((t) => t.plugin_id === pluginId),
    [mcpTools, pluginId],
  );

  // Extension operations
  const selectedExtension = useMemo(
    () => installedExtensions.find((e) => e.id === extensionId),
    [installedExtensions, extensionId],
  );

  // Filter handlers
  const handleFilterChange = useCallback(
    (idx: number, field: keyof SimpleFilter, val: string) => {
      setFilters((prev) => {
        const next = [...prev];
        next[idx] = { ...next[idx], [field]: val };
        return next;
      });
    },
    [],
  );

  const handleAddFilter = useCallback(() => {
    setFilters((prev) => [...prev, { dialect: "exact", attribute: "type", value: "" }]);
  }, []);

  const handleRemoveFilter = useCallback((idx: number) => {
    setFilters((prev) => {
      if (prev.length <= 1) return prev;
      return prev.filter((_, i) => i !== idx);
    });
  }, []);

  const toggleTemplateHelp = useCallback(() => {
    setShowTemplateHelp((p) => !p);
  }, []);

  // Build action from form state
  const buildAction = useCallback((): RouteAction | null => {
    let parsedArgs: Record<string, unknown> | undefined;
    if (argsTemplate.trim()) {
      try {
        parsedArgs = JSON.parse(argsTemplate);
      } catch {
        return null; // invalid JSON
      }
    }

    if (actionType === "invoke_plugin_tool") {
      if (!pluginId || !toolName) return null;
      return {
        action: "invoke_plugin_tool",
        plugin_id: pluginId,
        tool_name: toolName,
        ...(parsedArgs ? { args_template: parsedArgs } : {}),
      };
    }
    if (actionType === "call_extension") {
      if (!extensionId || !operation) return null;
      return {
        action: "call_extension",
        extension_id: extensionId,
        operation,
        ...(parsedArgs ? { args_template: parsedArgs } : {}),
      };
    }
    if (!channel) return null;
    return { action: "emit_frontend", channel };
  }, [actionType, pluginId, toolName, extensionId, operation, channel, argsTemplate]);

  const handleSave = useCallback(async () => {
    const action = buildAction();
    if (!action) return;

    setSaving(true);
    try {
      const ceFilters = fromSimpleFilters(filters);
      if (isEdit && rule) {
        await api.workflowUpdate({
          ruleId: rule.id,
          name: name || null,
          filters: ceFilters,
          action,
        });
      } else {
        await api.workflowCreate({
          name: name || undefined,
          filters: ceFilters,
          action,
        });
      }
      onSave();
    } catch {
      // silent
    } finally {
      setSaving(false);
    }
  }, [buildAction, filters, isEdit, rule, name, onSave]);

  const canSave = !!buildAction();

  // Memoized props for Select components
  const actionTypeKeys = useMemo(() => [actionType], [actionType]);
  const actionTypeChange = useCallback((keys: "all" | Set<string | number>) => {
    const val = keys === "all" ? undefined : (Array.from(keys)[0] as string);
    if (val) setActionType(val as RouteAction["action"]);
  }, []);

  const pluginIdKeys = useMemo(() => (pluginId ? [pluginId] : []), [pluginId]);
  const pluginIdChange = useCallback((keys: "all" | Set<string | number>) => {
    const val = keys === "all" ? undefined : (Array.from(keys)[0] as string);
    setPluginId(val ?? "");
    setToolName("");
  }, []);

  const toolNameKeys = useMemo(() => (toolName ? [toolName] : []), [toolName]);
  const toolNameChange = useCallback((keys: "all" | Set<string | number>) => {
    const val = keys === "all" ? undefined : (Array.from(keys)[0] as string);
    setToolName(val ?? "");
  }, []);

  const extensionIdKeys = useMemo(() => (extensionId ? [extensionId] : []), [extensionId]);
  const extensionIdChange = useCallback((keys: "all" | Set<string | number>) => {
    const val = keys === "all" ? undefined : (Array.from(keys)[0] as string);
    setExtensionId(val ?? "");
    setOperation("");
  }, []);

  const operationKeys = useMemo(() => (operation ? [operation] : []), [operation]);
  const operationChange = useCallback((keys: "all" | Set<string | number>) => {
    const val = keys === "all" ? undefined : (Array.from(keys)[0] as string);
    setOperation(val ?? "");
  }, []);

  const textareaClassNames = useMemo(() => ({ input: "font-mono text-xs" }), []);

  return (
    <div className="max-w-3xl mx-auto p-6 space-y-6">
      <div className="flex items-center gap-3">
        <Button variant="light" size="sm" onPress={onCancel}>
          {t("common:action.back")}
        </Button>
        <h1 className="text-xl font-bold">
          {isEdit ? t("plugins:workflows.editWorkflow") : t("plugins:workflows.createWorkflow")}
        </h1>
      </div>

      {/* Name */}
      <Surface className="p-4 space-y-4">
        <Input
          label={t("plugins:workflows.name")}
          placeholder={t("plugins:workflows.namePlaceholder")}
          value={name}
          onValueChange={setName}
        />
      </Surface>

      {/* Trigger / Filters */}
      <Surface className="p-4 space-y-4">
        <div>
          <h3 className="text-sm font-semibold mb-1">{t("plugins:workflows.trigger")}</h3>
          <p className="text-xs text-default-500">{t("plugins:workflows.triggerHint")}</p>
        </div>

        {/* eslint-disable react-perf/jsx-no-new-array-as-prop, react-perf/jsx-no-new-function-as-prop -- index-dependent callbacks in dynamic list */}
        {filters.map((f, idx) => (
          <div key={idx} className="flex items-end gap-2">
            <Select
              label={t("plugins:workflows.filterDialect")}
              selectedKeys={[f.dialect]}
              onSelectionChange={(keys) => {
                const val = Array.from(keys)[0] as string;
                if (val) handleFilterChange(idx, "dialect", val);
              }}
              className="w-32"
              size="sm"
            >
              <SelectItem key="exact">{t("plugins:workflows.exact")}</SelectItem>
              <SelectItem key="prefix">{t("plugins:workflows.prefix")}</SelectItem>
              <SelectItem key="suffix">{t("plugins:workflows.suffix")}</SelectItem>
            </Select>
            <Input
              label={t("plugins:workflows.filterAttribute")}
              placeholder="type"
              value={f.attribute}
              onValueChange={(v) => handleFilterChange(idx, "attribute", v)}
              size="sm"
              className="flex-1"
            />
            <Input
              label={t("plugins:workflows.filterValue")}
              placeholder="com.github."
              value={f.value}
              onValueChange={(v) => handleFilterChange(idx, "value", v)}
              size="sm"
              className="flex-1"
            />
            {filters.length > 1 && (
              <Button
                isIconOnly
                size="sm"
                variant="light"
                color="danger"
                onPress={() => handleRemoveFilter(idx)}
              >
                <X size={14} />
              </Button>
            )}
          </div>
        ))}
        {/* eslint-enable react-perf/jsx-no-new-array-as-prop, react-perf/jsx-no-new-function-as-prop */}
        <Button
          size="sm"
          variant="flat"
          startContent={<Plus size={14} />}
          onPress={handleAddFilter}
        >
          {t("plugins:workflows.addFilter")}
        </Button>
      </Surface>

      {/* Action */}
      <Surface className="p-4 space-y-4">
        <h3 className="text-sm font-semibold">{t("plugins:workflows.action")}</h3>

        <Select
          label={t("plugins:workflows.actionType")}
          selectedKeys={actionTypeKeys}
          onSelectionChange={actionTypeChange}
          size="sm"
        >
          <SelectItem key="invoke_plugin_tool">
            {t("plugins:workflows.actionPluginTool")}
          </SelectItem>
          <SelectItem key="call_extension">
            {t("plugins:workflows.actionExtensionOp")}
          </SelectItem>
          <SelectItem key="emit_frontend">
            {t("plugins:workflows.actionFrontendEvent")}
          </SelectItem>
        </Select>

        {actionType === "invoke_plugin_tool" && (
          <>
            <Select
              label={t("plugins:workflows.pluginId")}
              selectedKeys={pluginIdKeys}
              onSelectionChange={pluginIdChange}
              size="sm"
            >
              {pluginIds.map((id) => (
                <SelectItem key={id}>{id}</SelectItem>
              ))}
            </Select>
            {pluginId && (
              <Select
                label={t("plugins:workflows.toolName")}
                selectedKeys={toolNameKeys}
                onSelectionChange={toolNameChange}
                size="sm"
              >
                {pluginTools.map((tool) => (
                  <SelectItem key={tool.name}>{tool.name}</SelectItem>
                ))}
              </Select>
            )}
          </>
        )}

        {actionType === "call_extension" && (
          <>
            <Select
              label={t("plugins:workflows.extensionId")}
              selectedKeys={extensionIdKeys}
              onSelectionChange={extensionIdChange}
              size="sm"
            >
              {installedExtensions
                .filter((e) => e.enabled)
                .map((e) => (
                  <SelectItem key={e.id}>{e.display_name}</SelectItem>
                ))}
            </Select>
            {selectedExtension && (
              <Select
                label={t("plugins:workflows.operationName")}
                selectedKeys={operationKeys}
                onSelectionChange={operationChange}
                size="sm"
              >
                {selectedExtension.operations.map((op) => (
                  <SelectItem key={op.name}>{op.name}</SelectItem>
                ))}
              </Select>
            )}
          </>
        )}

        {actionType === "emit_frontend" && (
          <Input
            label={t("plugins:workflows.channelName")}
            placeholder={t("plugins:workflows.channelPlaceholder")}
            value={channel}
            onValueChange={setChannel}
            size="sm"
          />
        )}

        {/* Args template for plugin tool and extension actions */}
        {actionType !== "emit_frontend" && (
          <>
            <Textarea
              label={t("plugins:workflows.argsTemplate")}
              placeholder={t("plugins:workflows.argsTemplatePlaceholder")}
              value={argsTemplate}
              onValueChange={setArgsTemplate}
              size="sm"
              minRows={3}
              classNames={textareaClassNames}
            />
            <button
              onClick={toggleTemplateHelp}
              className="flex items-center gap-1 text-xs text-primary hover:underline"
            >
              <Info size={12} />
              {t("plugins:workflows.templateSyntax")}
              {showTemplateHelp ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
            </button>
            {showTemplateHelp && (
              <p className="text-xs text-default-500 bg-default-50 rounded-xl p-3">
                {t("plugins:workflows.templateSyntaxHint")}
              </p>
            )}
          </>
        )}
      </Surface>

      {/* Save / Cancel */}
      <div className="flex justify-end gap-3">
        <Button variant="flat" onPress={onCancel}>
          {t("common:action.cancel")}
        </Button>
        <Button
          color="primary"
          onPress={handleSave}
          isLoading={saving}
          isDisabled={!canSave}
        >
          {t("common:action.save")}
        </Button>
      </div>
    </div>
  );
}
