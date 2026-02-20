import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useExtensionActions } from "../../hooks/useExtensions";
import { useAppStore } from "../../stores/appStore";
import { ResourcePanel } from "../extensions/resources/ResourcePanel";
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
import {
  Button,
  Chip,
  Card,
  CardBody,
  Tooltip,
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
} from "@heroui/react";

const RISK_COLOR: Record<string, "success" | "warning" | "danger"> = {
  low: "success",
  medium: "warning",
  high: "danger",
};

export function ExtensionsTab() {
  const { t } = useTranslation("settings");
  const extensions = useAppStore((s) => s.installedExtensions);
  const busyExtensions = useAppStore((s) => s.busyExtensions);
  const focusExtensionId = useAppStore((s) => s.focusExtensionId);
  const { enable, disable, remove } = useExtensionActions();
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [prevFocusId, setPrevFocusId] = useState<string | null>(null);
  const [removeTarget, setRemoveTarget] = useState<string | null>(null);

  // Adjust state during render: auto-expand the deep-linked extension
  if (focusExtensionId && focusExtensionId !== prevFocusId) {
    setPrevFocusId(focusExtensionId);
    setExpanded((prev) => new Set(prev).add(focusExtensionId));
    useAppStore.getState().setFocusExtensionId(null);
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
      <Card><CardBody className="p-5">
        <div className="flex items-start justify-between">
          <div>
            <div className="flex items-center gap-2 mb-2">
              <Blocks size={15} strokeWidth={1.5} className="text-default-500" />
              <h3 className="text-[14px] font-semibold">
                {t("extensionsTab.hostExtensions")}
              </h3>
            </div>
            <p className="text-[11px] text-default-400">
              {t("extensionsTab.extensionsDesc")}
            </p>
            <div className="mt-3 flex items-center gap-2">
              <span className="text-[11px] text-default-500 font-medium">
                {t("extensionsTab.extensionsCount", { count: extensions.length })}
              </span>
            </div>
          </div>
          <Button
            onPress={() => useAppStore.getState().setView("extension-marketplace")}
            className="flex-shrink-0 ml-4"
          >
            <Plus size={12} strokeWidth={1.5} />
            {t("extensionsTab.addExtension")}
          </Button>
        </div>
      </CardBody></Card>

      {/* Extension cards */}
      {extensions.length === 0 ? (
        <Card><CardBody className="p-5">
          <p className="text-[12px] text-default-400">
            {t("extensionsTab.noExtensions")}
          </p>
        </CardBody></Card>
      ) : (
        extensions.map((ext) => {
          const isOpen = expanded.has(ext.id);
          const isBusy = !!busyExtensions[ext.id];
          return (
            <Card key={ext.id} className="overflow-hidden">
              {/* Extension header */}
              <button
                onClick={() => toggleExpanded(ext.id)}
                className="w-full flex items-center justify-between p-5 hover:bg-default-200/20 transition-colors"
              >
                <div className="min-w-0 flex-1 text-left">
                  <div className="flex items-center gap-2 mb-1">
                    <h4 className="text-[13px] font-semibold">
                      {ext.display_name}
                    </h4>
                    <span className="text-[10px] text-default-400 font-mono">
                      {ext.id}
                    </span>
                    {ext.installed && (
                      <Chip
                        size="sm"
                        variant="flat"
                        color={ext.enabled ? "success" : "default"}
                                              >
                        {ext.enabled ? t("common:status.enabled") : t("common:status.disabled")}
                      </Chip>
                    )}
                  </div>
                  <p className="text-[11px] text-default-400">
                    {ext.description}
                  </p>
                  <div className="flex items-center gap-3 mt-2">
                    <span className="text-[10px] text-default-500">
                      {t("extensionsTab.operationCount", { count: ext.operations.length })}
                    </span>
                    {ext.consumers.length > 0 && (
                      <span className="text-[10px] text-default-500">
                        {t("extensionsTab.pluginCount", { count: ext.consumers.length })}
                      </span>
                    )}
                  </div>
                </div>
                <ChevronDown
                  size={14}
                  strokeWidth={1.5}
                  className={`text-default-400 transition-transform duration-200 flex-shrink-0 ml-3 ${
                    isOpen ? "rotate-180" : ""
                  }`}
                />
              </button>

              {/* Expanded detail */}
              {isOpen && (
                <div>
                  {/* Enable/Disable + Remove controls */}
                  {ext.installed && (
                    <div className="px-4 pt-4 flex items-center gap-2">
                      <Button
                        onPress={() => {
                          if (ext.enabled) disable(ext.id);
                          else enable(ext.id);
                        }}
                        isDisabled={isBusy}
                      >
                        {isBusy ? (
                          <Loader2 size={12} strokeWidth={1.5} className="animate-spin" />
                        ) : (
                          <Power size={12} strokeWidth={1.5} />
                        )}
                        {ext.enabled ? t("common:action.disable") : t("common:action.enable")}
                      </Button>
                      <Button
                        isDisabled={isBusy}
                        color="danger"
                        onPress={() => setRemoveTarget(ext.id)}
                      >
                        <Trash2 size={12} strokeWidth={1.5} />
                        {t("common:action.remove")}
                      </Button>
                      <Modal
                        isOpen={removeTarget === ext.id}
                        onOpenChange={(open) => { if (!open) setRemoveTarget(null); }}
                      >
                        <ModalContent>
                          {(onClose) => (
                            <>
                              <ModalHeader className="text-[14px] flex items-center gap-2">
                                {t("common:confirm.removeExtension", { name: ext.display_name })}
                              </ModalHeader>
                              <ModalBody>
                                {ext.consumers.length > 0 ? (
                                  <>
                                    <p className="text-[13px] leading-relaxed text-default-500">
                                      {t("common:confirm.removeExtensionConsumers", { count: ext.consumers.length })}
                                    </p>
                                    <ul className="mt-2 space-y-1.5">
                                      {ext.consumers.map((c) => (
                                        <li
                                          key={c.plugin_id}
                                          className="flex items-center gap-2 px-3 py-2 rounded-[8px] bg-background border border-default-100"
                                        >
                                          <Puzzle size={12} strokeWidth={1.5} className="text-default-400 flex-shrink-0" />
                                          <span className="text-[12px] font-medium truncate">
                                            {c.plugin_name}
                                          </span>
                                          <span className="text-[10px] text-default-400 font-mono truncate ml-auto">
                                            {c.plugin_id}
                                          </span>
                                        </li>
                                      ))}
                                    </ul>
                                  </>
                                ) : (
                                  <p className="text-[13px] leading-relaxed text-default-500">
                                    {t("common:confirm.removeExtensionNoConsumers")}
                                  </p>
                                )}
                              </ModalBody>
                              <ModalFooter>
                                <Button variant="flat" onPress={onClose}>
                                  {t("common:action.cancel")}
                                </Button>
                                <Button
                                  color="danger"
                                  onPress={() => {
                                    remove(ext.id);
                                    onClose();
                                  }}
                                                                  >
                                  {t("common:confirm.removeExtensionAction")}
                                </Button>
                              </ModalFooter>
                            </>
                          )}
                        </ModalContent>
                      </Modal>
                    </div>
                  )}

                  {/* Operations */}
                  <div className="p-4">
                    <div className="flex items-center gap-2 mb-3">
                      <Blocks
                        size={12}
                        strokeWidth={1.5}
                        className="text-default-400"
                      />
                      <span className="text-[11px] font-semibold text-default-500 uppercase tracking-wide">
                        {t("extensionsTab.operations")}
                      </span>
                    </div>
                    <div className="space-y-1">
                      {ext.operations.map((op) => (
                        <div
                          key={op.name}
                          className="flex items-center gap-3 px-3 py-2 rounded-[8px] bg-background border border-default-100"
                        >
                          <span className="text-[12px] font-mono min-w-0 flex-shrink-0">
                            {op.name}
                          </span>
                          <Chip
                            size="sm"
                            variant="flat"
                            color={RISK_COLOR[op.risk_level] ?? "warning"}
                                                      >
                            {op.risk_level}
                          </Chip>
                          {op.scope_key && (
                            <Chip
                              size="sm"
                              variant="bordered"
                                                          >
                              scope: {op.scope_key}
                            </Chip>
                          )}
                          <span className="text-[11px] text-default-400 truncate min-w-0 flex-1">
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
                          className="text-default-400"
                        />
                        <span className="text-[11px] font-semibold text-default-500 uppercase tracking-wide">
                          {t("extensionsTab.capabilities")}
                        </span>
                      </div>
                      <div className="flex gap-1.5 flex-wrap">
                        {ext.capabilities.map((cap) => (
                          <Chip key={cap.type === "custom" ? cap.name : cap.type} size="sm">
                            {cap.type === "custom" ? cap.name : cap.type.replace(/_/g, " ")}
                          </Chip>
                        ))}
                      </div>
                    </div>
                  )}

                  {/* Resources (if extension declares any and is enabled) */}
                  {ext.enabled && Object.keys(ext.resources || {}).length > 0 && (
                    <div className="px-4 pb-4">
                      <ResourcePanel extension={ext} />
                    </div>
                  )}

                  {/* Plugin consumers */}
                  <div className="px-4 pb-4">
                    <div className="flex items-center gap-2 mb-3">
                      <Puzzle
                        size={12}
                        strokeWidth={1.5}
                        className="text-default-400"
                      />
                      <span className="text-[11px] font-semibold text-default-500 uppercase tracking-wide">
                        {t("extensionsTab.pluginConsumers")}
                      </span>
                    </div>
                    {ext.consumers.length === 0 ? (
                      <p className="text-[11px] text-default-400 px-3">
                        {t("extensionsTab.noConsumers")}
                      </p>
                    ) : (
                      <div className="space-y-1">
                        {ext.consumers.map((consumer) => (
                          <div
                            key={consumer.plugin_id}
                            className="flex items-center gap-3 px-3 py-2 rounded-[8px] bg-background border border-default-100"
                          >
                            <span className="text-[12px] font-medium truncate flex-1">
                              {consumer.plugin_name}
                            </span>
                            <Tooltip
                              content={
                                consumer.granted
                                  ? t("extensionsTab.allPermsGranted")
                                  : t("extensionsTab.somePermsMissing")
                              }
                              size="sm"
                            >
                              <span className="flex-shrink-0">
                                {consumer.granted ? (
                                  <Shield
                                    size={12}
                                    strokeWidth={1.5}
                                    className="text-success cursor-help"
                                  />
                                ) : (
                                  <ShieldAlert
                                    size={12}
                                    strokeWidth={1.5}
                                    className="text-warning cursor-help"
                                  />
                                )}
                              </span>
                            </Tooltip>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                </div>
              )}
            </Card>
          );
        })
      )}
    </div>
  );
}
