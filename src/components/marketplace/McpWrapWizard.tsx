import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { ClassifiedTool, PluginMetadata } from "../../types/mcp_wrap";
import type { Permission } from "../../types/permissions";
import { getPermissionInfo } from "../../types/permissions";
import {
  mcpDiscoverTools,
  mcpSuggestMetadata,
  mcpGenerateAndInstall,
} from "../../lib/tauri";
import {
  X,
  ArrowLeft,
  ArrowRight,
  Terminal,
  Wrench,
  AlertTriangle,
  ShieldCheck,
  Loader2,
  Check,
  Eye,
  EyeOff,
} from "lucide-react";
import {
  Modal,
  ModalContent,
  ModalBody,
  Switch,
  Button,
  Input,
  Chip,
} from "@heroui/react";

const riskChipColors: Record<string, "success" | "warning" | "danger"> = {
  low: "success",
  medium: "warning",
  high: "danger",
};

type Step = "command" | "tools" | "details" | "permissions" | "build";

interface Props {
  onClose: () => void;
  onInstalled: () => void;
}

export function McpWrapWizard({ onClose, onInstalled }: Props) {
  const { t } = useTranslation("plugins");
  const [step, setStep] = useState<Step>("command");

  // Step 1: Command
  const [command, setCommand] = useState("");
  const [discovering, setDiscovering] = useState(false);
  const [discoverError, setDiscoverError] = useState<string | null>(null);

  // Step 2: Tools
  const [tools, setTools] = useState<ClassifiedTool[]>([]);
  const [includedTools, setIncludedTools] = useState<Set<string>>(new Set());

  // Step 3: Metadata
  const [metadata, setMetadata] = useState<PluginMetadata>({
    id: "",
    name: "",
    description: "",
    author: "",
  });

  // Step 4: Permissions
  const [permToggles, setPermToggles] = useState<Record<string, boolean>>({});

  // Step 5: Build
  const [buildPhase, setBuildPhase] = useState("");
  const [building, setBuilding] = useState(false);
  const [buildError, setBuildError] = useState<string | null>(null);
  const [buildSuccess, setBuildSuccess] = useState(false);

  // -- Step 1: Discover --

  async function handleDiscover() {
    setDiscoverError(null);
    setDiscovering(true);
    try {
      const [classified, suggested] = await Promise.all([
        mcpDiscoverTools(command),
        mcpSuggestMetadata(command),
      ]);
      if (classified.length === 0) {
        setDiscoverError(t("mcpWrap.zeroTools"));
        return;
      }
      setTools(classified);
      setIncludedTools(new Set(classified.map((t) => t.name)));
      setMetadata(suggested);

      // Initialize permission toggles from union of all tool permissions
      const allPerms = new Set<string>();
      for (const tool of classified) {
        for (const p of tool.permissions) allPerms.add(p);
      }
      const toggles: Record<string, boolean> = {};
      for (const p of allPerms) toggles[p] = true;
      setPermToggles(toggles);

      setStep("tools");
    } catch (err) {
      setDiscoverError(String(err));
    } finally {
      setDiscovering(false);
    }
  }

  // -- Step 5: Build --

  async function handleBuild() {
    setBuilding(true);
    setBuildError(null);
    setBuildSuccess(false);

    const selectedTools = tools.filter((t) => includedTools.has(t.name));
    const approved: Permission[] = [];
    const deferred: Permission[] = [];
    for (const [perm, on] of Object.entries(permToggles)) {
      if (on) approved.push(perm as Permission);
      else deferred.push(perm as Permission);
    }

    try {
      setBuildPhase(t("mcpWrap.generating"));
      // Small delay so the user sees the phase text
      await new Promise((r) => setTimeout(r, 100));

      setBuildPhase(t("mcpWrap.buildingContainer"));
      await mcpGenerateAndInstall(
        command,
        selectedTools,
        metadata,
        approved,
        deferred
      );

      setBuildPhase(t("mcpWrap.done"));
      setBuildSuccess(true);
    } catch (err) {
      setBuildError(String(err));
    } finally {
      setBuilding(false);
    }
  }

  // -- Navigation --

  const steps: { id: Step; label: string }[] = [
    { id: "command", label: t("mcpWrap.stepCommand") },
    { id: "tools", label: t("mcpWrap.stepTools") },
    { id: "details", label: t("mcpWrap.stepDetails") },
    { id: "permissions", label: t("mcpWrap.stepPermissions") },
    { id: "build", label: t("mcpWrap.stepBuild") },
  ];

  // -- Render --

  return (
    <Modal
      isOpen
      onOpenChange={(open) => { if (!open) onClose(); }}
      hideCloseButton
    >
      <ModalContent>
        {() => (
          <>
            {/* Header */}
            <div className="flex items-center justify-between px-6 pt-5 pb-3">
              <h2 className="text-[16px] font-bold">
                {t("mcpWrap.title")}
              </h2>
              <Button
                isIconOnly
                onPress={onClose}
              >
                <X size={16} strokeWidth={1.5} />
              </Button>
            </div>

            {/* Step indicator */}
            <div className="flex border-b border-default-100">
              {steps.map((s) => (
                <div
                  key={s.id}
                  className={`flex-1 px-2 py-2 text-[10px] font-semibold text-center uppercase tracking-wider transition-colors duration-150 ${
                    step === s.id
                      ? "text-primary border-b-2 border-primary"
                      : "text-default-400"
                  }`}
                >
                  {s.label}
                </div>
              ))}
            </div>

            {/* Content */}
            <ModalBody className="p-6">
              {step === "command" && (
                <CommandStep
                  command={command}
                  setCommand={setCommand}
                  discovering={discovering}
                  error={discoverError}
                  onDiscover={handleDiscover}
                  onClose={onClose}
                />
              )}
              {step === "tools" && (
                <ToolsStep
                  tools={tools}
                  includedTools={includedTools}
                  setIncludedTools={setIncludedTools}
                  onBack={() => setStep("command")}
                  onNext={() => setStep("details")}
                />
              )}
              {step === "details" && (
                <DetailsStep
                  metadata={metadata}
                  setMetadata={setMetadata}
                  onBack={() => setStep("tools")}
                  onNext={() => setStep("permissions")}
                />
              )}
              {step === "permissions" && (
                <PermissionsStep
                  permToggles={permToggles}
                  setPermToggles={setPermToggles}
                  onBack={() => setStep("details")}
                  onNext={() => {
                    setStep("build");
                    handleBuild();
                  }}
                />
              )}
              {step === "build" && (
                <BuildStep
                  phase={buildPhase}
                  building={building}
                  error={buildError}
                  success={buildSuccess}
                  onRetry={handleBuild}
                  onDone={() => {
                    onInstalled();
                    onClose();
                  }}
                />
              )}
            </ModalBody>
          </>
        )}
      </ModalContent>
    </Modal>
  );
}

// -- Step Components --

function CommandStep({
  command,
  setCommand,
  discovering,
  error,
  onDiscover,
  onClose,
}: {
  command: string;
  setCommand: (v: string) => void;
  discovering: boolean;
  error: string | null;
  onDiscover: () => void;
  onClose: () => void;
}) {
  const { t } = useTranslation("plugins");

  return (
    <>
      <p className="text-[13px] text-default-500 mb-4">
        {t("mcpWrap.commandDesc")}
      </p>

      <div className="mb-2">
        <label className="block text-[11px] font-medium text-default-500 mb-1.5 uppercase tracking-wider">
          {t("mcpWrap.mcpServerCommand")}
        </label>
        <Input
          value={command}
          onValueChange={setCommand}
          onKeyDown={(e) => {
            if (e.key === "Enter" && command.trim()) onDiscover();
          }}
          placeholder={t("mcpWrap.commandPlaceholder")}
          startContent={
            <Terminal
              size={14}
              strokeWidth={1.5}
              className="text-default-400"
            />
          }
          variant="bordered"
          autoFocus
        />
      </div>

      <p className="text-[11px] text-default-400 mb-4">
        {t("mcpWrap.supportedRuntimes", {
          interpolation: { escapeValue: false },
        })}
      </p>

      {error && (
        <div className="mb-4 p-3 rounded-[8px] bg-danger-50/50 border border-danger/20">
          <p className="text-[12px] text-danger">{error}</p>
        </div>
      )}

      <div className="flex gap-3 justify-end">
        <Button onPress={onClose}>
          {t("common:action.cancel")}
        </Button>
        <Button
          onPress={onDiscover}
          isDisabled={!command.trim() || discovering}
        >
          {discovering ? (
            <>
              <Loader2 size={14} strokeWidth={1.5} className="animate-spin" />
              {t("mcpWrap.discovering")}
            </>
          ) : (
            <>
              {t("mcpWrap.discoverTools")}
              <ArrowRight size={14} strokeWidth={1.5} />
            </>
          )}
        </Button>
      </div>
    </>
  );
}

function ToolsStep({
  tools,
  includedTools,
  setIncludedTools,
  onBack,
  onNext,
}: {
  tools: ClassifiedTool[];
  includedTools: Set<string>;
  setIncludedTools: (v: Set<string>) => void;
  onBack: () => void;
  onNext: () => void;
}) {
  const { t } = useTranslation("plugins");

  function toggleTool(name: string) {
    const next = new Set(includedTools);
    if (next.has(name)) next.delete(name);
    else next.add(name);
    setIncludedTools(next);
  }

  return (
    <>
      <p className="text-[13px] text-default-500 mb-4">
        {t("mcpWrap.toolsDiscovered", { count: tools.length })}
      </p>

      <div className="space-y-2 mb-5 max-h-64 overflow-y-auto">
        {tools.map((tool) => {
          const included = includedTools.has(tool.name);
          return (
            <div
              key={tool.name}
              className={`p-3 rounded-[8px] border transition-colors duration-150 cursor-pointer ${
                included
                  ? "bg-background border-default-100"
                  : "bg-background/40 border-default-100/40 opacity-50"
              }`}
              onClick={() => toggleTool(tool.name)}
            >
              <div className="flex items-center justify-between mb-1">
                <div className="flex items-center gap-2 min-w-0">
                  <Wrench
                    size={11}
                    strokeWidth={1.5}
                    className="text-default-500 flex-shrink-0"
                  />
                  <span className="text-[12px] font-medium font-mono truncate">
                    {tool.name}
                  </span>
                  {tool.high_risk && (
                    <AlertTriangle
                      size={12}
                      strokeWidth={1.5}
                      className="text-danger flex-shrink-0"
                    />
                  )}
                </div>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    toggleTool(tool.name);
                  }}
                  className="flex-shrink-0 ml-2 h-6 w-6 flex items-center justify-center"
                >
                  {included ? (
                    <Eye size={14} strokeWidth={1.5} className="text-primary" />
                  ) : (
                    <EyeOff size={14} strokeWidth={1.5} className="text-default-400" />
                  )}
                </button>
              </div>
              <p className="text-[11px] text-default-500 ml-[19px] line-clamp-2">
                {tool.description}
              </p>
              {tool.permissions.length > 0 && (
                <div className="flex flex-wrap gap-1 ml-[19px] mt-1.5">
                  {tool.permissions.map((perm) => {
                    const info = getPermissionInfo(perm);
                    return (
                      <Chip key={perm} size="sm" variant="flat" color={riskChipColors[info.risk]}>
                        {perm}
                      </Chip>
                    );
                  })}
                </div>
              )}
            </div>
          );
        })}
      </div>

      <div className="flex justify-between">
        <Button onPress={onBack}>
          <ArrowLeft size={14} strokeWidth={1.5} />
          {t("common:action.back")}
        </Button>
        <Button
          onPress={onNext}
          isDisabled={includedTools.size === 0}
        >
          {t("common:action.continue")}
          <ArrowRight size={14} strokeWidth={1.5} />
        </Button>
      </div>
    </>
  );
}

function DetailsStep({
  metadata,
  setMetadata,
  onBack,
  onNext,
}: {
  metadata: PluginMetadata;
  setMetadata: (v: PluginMetadata) => void;
  onBack: () => void;
  onNext: () => void;
}) {
  const { t } = useTranslation("plugins");
  const idValid = /^[a-z0-9][a-z0-9.-]*$/.test(metadata.id);

  return (
    <>
      <p className="text-[13px] text-default-500 mb-4">
        {t("mcpWrap.reviewMetadata")}
      </p>

      <div className="space-y-3 mb-5">
        <FieldInput
          label={t("mcpWrap.pluginId")}
          value={metadata.id}
          onChange={(v) => setMetadata({ ...metadata, id: v })}
          mono
          placeholder={t("mcpWrap.pluginIdPlaceholder")}
          error={metadata.id && !idValid ? t("mcpWrap.idValidation") : undefined}
        />
        <FieldInput
          label={t("mcpWrap.displayName")}
          value={metadata.name}
          onChange={(v) => setMetadata({ ...metadata, name: v })}
          placeholder={t("mcpWrap.displayNamePlaceholder")}
        />
        <FieldInput
          label={t("mcpWrap.description")}
          value={metadata.description}
          onChange={(v) => setMetadata({ ...metadata, description: v })}
          placeholder={t("mcpWrap.descriptionPlaceholder")}
        />
        <FieldInput
          label={t("mcpWrap.author")}
          value={metadata.author}
          onChange={(v) => setMetadata({ ...metadata, author: v })}
          placeholder={t("mcpWrap.authorPlaceholder")}
        />
      </div>

      <div className="flex justify-between">
        <Button onPress={onBack}>
          <ArrowLeft size={14} strokeWidth={1.5} />
          {t("common:action.back")}
        </Button>
        <Button
          onPress={onNext}
          isDisabled={!metadata.id || !metadata.name || !idValid}
        >
          {t("common:action.continue")}
          <ArrowRight size={14} strokeWidth={1.5} />
        </Button>
      </div>
    </>
  );
}

function PermissionsStep({
  permToggles,
  setPermToggles,
  onBack,
  onNext,
}: {
  permToggles: Record<string, boolean>;
  setPermToggles: (v: Record<string, boolean>) => void;
  onBack: () => void;
  onNext: () => void;
}) {
  const { t } = useTranslation("plugins");
  const permissions = Object.keys(permToggles);
  const deferredCount = permissions.filter((p) => !permToggles[p]).length;

  function toggle(perm: string) {
    setPermToggles({ ...permToggles, [perm]: !permToggles[perm] });
  }

  return (
    <>
      <p className="text-[13px] text-default-500 mb-1">
        {t("mcpWrap.reviewPermissions")}
      </p>
      {deferredCount > 0 && (
        <p className="text-[11px] text-warning mb-4">
          {t("mcpWrap.deferredCount", { count: deferredCount })}
        </p>
      )}
      {deferredCount === 0 && <div className="mb-4" />}

      {permissions.length === 0 ? (
        <p className="text-[12px] text-default-400 mb-5">
          {t("mcpWrap.noPermissionsRequired")}
        </p>
      ) : (
        <div className="space-y-2 mb-5 max-h-52 overflow-y-auto">
          {permissions.map((perm) => {
            const info = getPermissionInfo(perm);
            const enabled = permToggles[perm];
            return (
              <div
                key={perm}
                className={`flex items-center justify-between p-3 rounded-[8px] border transition-colors duration-150 ${
                  enabled
                    ? "bg-background border-default-100"
                    : "bg-background/50 border-default-100/50 opacity-60"
                }`}
              >
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <p className="text-[12px] font-medium font-mono">
                      {perm}
                    </p>
                    <Chip size="sm" variant="flat" color={riskChipColors[info.risk]}>
                      {info.risk}
                    </Chip>
                  </div>
                  <p className="text-[11px] text-default-500 mt-0.5">
                    {info.description}
                  </p>
                </div>
                <Switch
                  isSelected={enabled}
                  onValueChange={() => toggle(perm)}
                  className="ml-3"
                />
              </div>
            );
          })}
        </div>
      )}

      <div className="flex justify-between">
        <Button onPress={onBack}>
          <ArrowLeft size={14} strokeWidth={1.5} />
          {t("common:action.back")}
        </Button>
        <Button onPress={onNext}>
          <ShieldCheck size={14} strokeWidth={1.5} />
          {t("marketplace.buildAndInstall")}
        </Button>
      </div>
    </>
  );
}

function BuildStep({
  phase,
  building,
  error,
  success,
  onRetry,
  onDone,
}: {
  phase: string;
  building: boolean;
  error: string | null;
  success: boolean;
  onRetry: () => void;
  onDone: () => void;
}) {
  const { t } = useTranslation("plugins");

  return (
    <div className="text-center py-6">
      {building && (
        <>
          <Loader2
            size={32}
            strokeWidth={1.5}
            className="animate-spin text-primary mx-auto mb-4"
          />
          <p className="text-[14px] font-medium mb-1">{phase}</p>
          <p className="text-[12px] text-default-500">
            {t("mcpWrap.firstBuildNote")}
          </p>
        </>
      )}

      {error && !building && (
        <>
          <div className="w-12 h-12 rounded-full bg-danger-50 flex items-center justify-center mx-auto mb-4">
            <AlertTriangle size={22} strokeWidth={1.5} className="text-danger" />
          </div>
          <p className="text-[14px] font-medium mb-2">
            {t("mcpWrap.buildFailed")}
          </p>
          <p className="text-[12px] text-danger mb-5 max-w-sm mx-auto break-words">
            {error}
          </p>
          <Button onPress={onRetry}>
            {t("mcpWrap.tryAgain")}
          </Button>
        </>
      )}

      {success && (
        <>
          <div className="w-12 h-12 rounded-full bg-success-50 flex items-center justify-center mx-auto mb-4">
            <Check size={22} strokeWidth={1.5} className="text-success" />
          </div>
          <p className="text-[14px] font-medium mb-2">
            {t("mcpWrap.pluginInstalledTitle")}
          </p>
          <p className="text-[12px] text-default-500 mb-5">
            {t("mcpWrap.pluginInstalledDesc")}
          </p>
          <Button onPress={onDone} className="mx-auto">
            <Check size={14} strokeWidth={1.5} />
            {t("mcpWrap.goToPlugins")}
          </Button>
        </>
      )}
    </div>
  );
}

// -- Helpers --

function FieldInput({
  label,
  value,
  onChange,
  placeholder,
  error,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  error?: string;
}) {
  return (
    <div>
      <label className="block text-[11px] font-medium text-default-500 mb-1 uppercase tracking-wider">
        {label}
      </label>
      <Input
        value={value}
        onValueChange={onChange}
        placeholder={placeholder}
        color={error ? "danger" : undefined}
        variant="bordered"
      />
      {error && (
        <p className="text-[10px] text-danger mt-0.5">{error}</p>
      )}
    </div>
  );
}
