import { useState } from "react";
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

const riskColors = {
  low: "text-nx-success bg-nx-success-muted",
  medium: "text-nx-warning bg-nx-warning-muted",
  high: "text-nx-error bg-nx-error-muted",
};

type Step = "command" | "tools" | "details" | "permissions" | "build";

interface Props {
  onClose: () => void;
  onInstalled: () => void;
}

export function McpWrapWizard({ onClose, onInstalled }: Props) {
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

  // ── Step 1: Discover ──────────────────────────────────────────

  async function handleDiscover() {
    setDiscoverError(null);
    setDiscovering(true);
    try {
      const [classified, suggested] = await Promise.all([
        mcpDiscoverTools(command),
        mcpSuggestMetadata(command),
      ]);
      if (classified.length === 0) {
        setDiscoverError("MCP server reported 0 tools. Nothing to wrap.");
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

  // ── Step 5: Build ─────────────────────────────────────────────

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
      setBuildPhase("Generating plugin...");
      // Small delay so the user sees the phase text
      await new Promise((r) => setTimeout(r, 100));

      setBuildPhase("Building Docker image...");
      await mcpGenerateAndInstall(
        command,
        selectedTools,
        metadata,
        approved,
        deferred
      );

      setBuildPhase("Done!");
      setBuildSuccess(true);
    } catch (err) {
      setBuildError(String(err));
    } finally {
      setBuilding(false);
    }
  }

  // ── Navigation ────────────────────────────────────────────────

  const steps: { id: Step; label: string }[] = [
    { id: "command", label: "Command" },
    { id: "tools", label: "Tools" },
    { id: "details", label: "Details" },
    { id: "permissions", label: "Permissions" },
    { id: "build", label: "Build" },
  ];

  const selectedToolCount = includedTools.size;

  // ── Render ────────────────────────────────────────────────────

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div
        className="absolute inset-0 bg-black/50 backdrop-blur-sm"
        onClick={onClose}
      />
      <div
        className="relative bg-nx-surface border border-nx-border rounded-[var(--radius-modal)] shadow-[var(--shadow-modal)] max-w-lg w-full mx-4 overflow-hidden"
        style={{ animation: "toast-enter 200ms ease-out" }}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-6 pt-5 pb-3">
          <h3 className="text-[16px] font-bold text-nx-text">
            Wrap MCP Server
          </h3>
          <button
            onClick={onClose}
            className="p-1 rounded-[var(--radius-tag)] hover:bg-nx-wash text-nx-text-muted transition-colors"
          >
            <X size={16} strokeWidth={1.5} />
          </button>
        </div>

        {/* Step indicator */}
        <div className="flex border-b border-nx-border-subtle">
          {steps.map((s) => (
            <div
              key={s.id}
              className={`flex-1 px-2 py-2 text-[10px] font-semibold text-center uppercase tracking-wider transition-colors duration-150 ${
                step === s.id
                  ? "text-nx-accent border-b-2 border-nx-accent"
                  : "text-nx-text-ghost"
              }`}
            >
              {s.label}
            </div>
          ))}
        </div>

        {/* Content */}
        <div className="p-6">
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
        </div>
      </div>
    </div>
  );
}

// ── Step Components ───────────────────────────────────────────────

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
  return (
    <>
      <p className="text-[13px] text-nx-text-secondary mb-4">
        Enter the command used to start your MCP server. Nexus will discover its
        tools, infer permissions, and generate a headless plugin.
      </p>

      <div className="mb-2">
        <label className="block text-[11px] font-medium text-nx-text-muted mb-1.5 uppercase tracking-wider">
          MCP Server Command
        </label>
        <div className="relative">
          <Terminal
            size={14}
            strokeWidth={1.5}
            className="absolute left-3 top-1/2 -translate-y-1/2 text-nx-text-ghost"
          />
          <input
            type="text"
            value={command}
            onChange={(e) => setCommand(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && command.trim()) onDiscover();
            }}
            placeholder="npx -y @org/server-name"
            className="w-full pl-9 pr-3 py-2.5 text-[13px] font-mono bg-nx-deep border border-nx-border-subtle rounded-[var(--radius-button)] text-nx-text placeholder:text-nx-text-ghost focus:outline-none focus:border-nx-accent transition-colors"
            autoFocus
          />
        </div>
      </div>

      <p className="text-[11px] text-nx-text-ghost mb-4">
        Supported runtimes: <code className="text-nx-text-muted">npx</code>,{" "}
        <code className="text-nx-text-muted">node</code>
      </p>

      {error && (
        <div className="mb-4 p-3 rounded-[var(--radius-button)] bg-nx-error-muted/50 border border-nx-error/20">
          <p className="text-[12px] text-nx-error">{error}</p>
        </div>
      )}

      <div className="flex gap-3 justify-end">
        <button
          onClick={onClose}
          className="px-4 py-2 text-[13px] font-medium rounded-[var(--radius-button)] bg-nx-overlay hover:bg-nx-wash text-nx-text-secondary transition-all duration-150"
        >
          Cancel
        </button>
        <button
          onClick={onDiscover}
          disabled={!command.trim() || discovering}
          className="flex items-center gap-1.5 px-4 py-2 text-[13px] font-medium rounded-[var(--radius-button)] bg-nx-accent hover:bg-nx-accent-hover disabled:opacity-40 text-nx-deep transition-all duration-150"
        >
          {discovering ? (
            <>
              <Loader2 size={14} strokeWidth={1.5} className="animate-spin" />
              Discovering...
            </>
          ) : (
            <>
              Discover Tools
              <ArrowRight size={14} strokeWidth={1.5} />
            </>
          )}
        </button>
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
  function toggleTool(name: string) {
    const next = new Set(includedTools);
    if (next.has(name)) next.delete(name);
    else next.add(name);
    setIncludedTools(next);
  }

  return (
    <>
      <p className="text-[13px] text-nx-text-secondary mb-4">
        Discovered <span className="font-semibold text-nx-text">{tools.length}</span> tool{tools.length !== 1 ? "s" : ""}.
        Toggle tools to include or exclude them from the plugin.
      </p>

      <div className="space-y-2 mb-5 max-h-64 overflow-y-auto">
        {tools.map((tool) => {
          const included = includedTools.has(tool.name);
          return (
            <div
              key={tool.name}
              className={`p-3 rounded-[var(--radius-button)] border transition-colors duration-150 cursor-pointer ${
                included
                  ? "bg-nx-deep border-nx-border-subtle"
                  : "bg-nx-deep/40 border-nx-border-subtle/40 opacity-50"
              }`}
              onClick={() => toggleTool(tool.name)}
            >
              <div className="flex items-center justify-between mb-1">
                <div className="flex items-center gap-2 min-w-0">
                  <Wrench
                    size={11}
                    strokeWidth={1.5}
                    className="text-nx-text-muted flex-shrink-0"
                  />
                  <span className="text-[12px] font-medium font-mono text-nx-text truncate">
                    {tool.name}
                  </span>
                  {tool.high_risk && (
                    <AlertTriangle
                      size={12}
                      strokeWidth={1.5}
                      className="text-nx-error flex-shrink-0"
                    />
                  )}
                </div>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    toggleTool(tool.name);
                  }}
                  className="flex-shrink-0 ml-2"
                >
                  {included ? (
                    <Eye size={14} strokeWidth={1.5} className="text-nx-accent" />
                  ) : (
                    <EyeOff size={14} strokeWidth={1.5} className="text-nx-text-ghost" />
                  )}
                </button>
              </div>
              <p className="text-[11px] text-nx-text-muted ml-[19px] line-clamp-2">
                {tool.description}
              </p>
              {tool.permissions.length > 0 && (
                <div className="flex flex-wrap gap-1 ml-[19px] mt-1.5">
                  {tool.permissions.map((perm) => {
                    const info = getPermissionInfo(perm);
                    return (
                      <span
                        key={perm}
                        className={`text-[9px] font-medium px-1.5 py-0.5 rounded-[var(--radius-tag)] ${riskColors[info.risk]}`}
                      >
                        {perm}
                      </span>
                    );
                  })}
                </div>
              )}
            </div>
          );
        })}
      </div>

      <div className="flex justify-between">
        <button
          onClick={onBack}
          className="flex items-center gap-1.5 px-4 py-2 text-[13px] font-medium rounded-[var(--radius-button)] text-nx-text-muted hover:text-nx-text-secondary transition-colors duration-150"
        >
          <ArrowLeft size={14} strokeWidth={1.5} />
          Back
        </button>
        <button
          onClick={onNext}
          disabled={includedTools.size === 0}
          className="flex items-center gap-1.5 px-4 py-2 text-[13px] font-medium rounded-[var(--radius-button)] bg-nx-accent hover:bg-nx-accent-hover disabled:opacity-40 text-nx-deep transition-all duration-150"
        >
          Continue
          <ArrowRight size={14} strokeWidth={1.5} />
        </button>
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
  const idValid = /^[a-z0-9][a-z0-9.\-]*$/.test(metadata.id);

  return (
    <>
      <p className="text-[13px] text-nx-text-secondary mb-4">
        Review and edit the plugin metadata. These are auto-filled from the
        server command.
      </p>

      <div className="space-y-3 mb-5">
        <FieldInput
          label="Plugin ID"
          value={metadata.id}
          onChange={(v) => setMetadata({ ...metadata, id: v })}
          mono
          placeholder="mcp.my-server"
          error={metadata.id && !idValid ? "Lowercase, dots, and dashes only" : undefined}
        />
        <FieldInput
          label="Display Name"
          value={metadata.name}
          onChange={(v) => setMetadata({ ...metadata, name: v })}
          placeholder="My MCP Server"
        />
        <FieldInput
          label="Description"
          value={metadata.description}
          onChange={(v) => setMetadata({ ...metadata, description: v })}
          placeholder="What this plugin does"
        />
        <FieldInput
          label="Author"
          value={metadata.author}
          onChange={(v) => setMetadata({ ...metadata, author: v })}
          placeholder="Your name"
        />
      </div>

      <div className="flex justify-between">
        <button
          onClick={onBack}
          className="flex items-center gap-1.5 px-4 py-2 text-[13px] font-medium rounded-[var(--radius-button)] text-nx-text-muted hover:text-nx-text-secondary transition-colors duration-150"
        >
          <ArrowLeft size={14} strokeWidth={1.5} />
          Back
        </button>
        <button
          onClick={onNext}
          disabled={!metadata.id || !metadata.name || !idValid}
          className="flex items-center gap-1.5 px-4 py-2 text-[13px] font-medium rounded-[var(--radius-button)] bg-nx-accent hover:bg-nx-accent-hover disabled:opacity-40 text-nx-deep transition-all duration-150"
        >
          Continue
          <ArrowRight size={14} strokeWidth={1.5} />
        </button>
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
  const permissions = Object.keys(permToggles);
  const deferredCount = permissions.filter((p) => !permToggles[p]).length;

  function toggle(perm: string) {
    setPermToggles({ ...permToggles, [perm]: !permToggles[perm] });
  }

  return (
    <>
      <p className="text-[13px] text-nx-text-secondary mb-1">
        Review the permissions this plugin will request.
      </p>
      {deferredCount > 0 && (
        <p className="text-[11px] text-nx-warning mb-4">
          {deferredCount} permission{deferredCount !== 1 ? "s" : ""} deferred —
          will prompt on first use
        </p>
      )}
      {deferredCount === 0 && <div className="mb-4" />}

      {permissions.length === 0 ? (
        <p className="text-[12px] text-nx-text-ghost mb-5">
          No permissions required.
        </p>
      ) : (
        <div className="space-y-2 mb-5 max-h-52 overflow-y-auto">
          {permissions.map((perm) => {
            const info = getPermissionInfo(perm);
            const enabled = permToggles[perm];
            return (
              <div
                key={perm}
                className={`flex items-center justify-between p-3 rounded-[var(--radius-button)] border transition-colors duration-150 ${
                  enabled
                    ? "bg-nx-deep border-nx-border-subtle"
                    : "bg-nx-deep/50 border-nx-border-subtle/50 opacity-60"
                }`}
              >
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <p className="text-[12px] text-nx-text font-medium font-mono">
                      {perm}
                    </p>
                    <span
                      className={`text-[10px] px-2 py-0.5 rounded-[var(--radius-tag)] font-semibold capitalize ${riskColors[info.risk]}`}
                    >
                      {info.risk}
                    </span>
                  </div>
                  <p className="text-[11px] text-nx-text-muted mt-0.5">
                    {info.description}
                  </p>
                </div>
                <button
                  onClick={() => toggle(perm)}
                  className={`relative ml-3 w-9 h-5 rounded-full flex-shrink-0 transition-colors duration-200 ${
                    enabled ? "bg-nx-accent" : "bg-nx-overlay"
                  }`}
                  title={
                    enabled
                      ? "Approved — click to defer"
                      : "Deferred — click to approve"
                  }
                >
                  <span
                    className={`absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform duration-200 ${
                      enabled ? "left-[18px]" : "left-0.5"
                    }`}
                  />
                </button>
              </div>
            );
          })}
        </div>
      )}

      <div className="flex justify-between">
        <button
          onClick={onBack}
          className="flex items-center gap-1.5 px-4 py-2 text-[13px] font-medium rounded-[var(--radius-button)] text-nx-text-muted hover:text-nx-text-secondary transition-colors duration-150"
        >
          <ArrowLeft size={14} strokeWidth={1.5} />
          Back
        </button>
        <button
          onClick={onNext}
          className="flex items-center gap-1.5 px-4 py-2 text-[13px] font-medium rounded-[var(--radius-button)] bg-nx-accent hover:bg-nx-accent-hover text-nx-deep transition-all duration-150"
        >
          <ShieldCheck size={14} strokeWidth={1.5} />
          Build & Install
        </button>
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
  return (
    <div className="text-center py-6">
      {building && (
        <>
          <Loader2
            size={32}
            strokeWidth={1.5}
            className="animate-spin text-nx-accent mx-auto mb-4"
          />
          <p className="text-[14px] font-medium text-nx-text mb-1">{phase}</p>
          <p className="text-[12px] text-nx-text-muted">
            This may take a minute for the first build...
          </p>
        </>
      )}

      {error && !building && (
        <>
          <div className="w-12 h-12 rounded-full bg-nx-error-muted flex items-center justify-center mx-auto mb-4">
            <AlertTriangle size={22} strokeWidth={1.5} className="text-nx-error" />
          </div>
          <p className="text-[14px] font-medium text-nx-text mb-2">
            Build Failed
          </p>
          <p className="text-[12px] text-nx-error mb-5 max-w-sm mx-auto break-words">
            {error}
          </p>
          <button
            onClick={onRetry}
            className="px-4 py-2 text-[13px] font-medium rounded-[var(--radius-button)] bg-nx-accent hover:bg-nx-accent-hover text-nx-deep transition-all duration-150"
          >
            Try Again
          </button>
        </>
      )}

      {success && (
        <>
          <div className="w-12 h-12 rounded-full bg-nx-success-muted flex items-center justify-center mx-auto mb-4">
            <Check size={22} strokeWidth={1.5} className="text-nx-success" />
          </div>
          <p className="text-[14px] font-medium text-nx-text mb-2">
            Plugin Installed
          </p>
          <p className="text-[12px] text-nx-text-muted mb-5">
            Your MCP server has been wrapped and installed as a headless Nexus
            plugin. Start it from the plugins page.
          </p>
          <button
            onClick={onDone}
            className="flex items-center gap-1.5 mx-auto px-4 py-2 text-[13px] font-medium rounded-[var(--radius-button)] bg-nx-accent hover:bg-nx-accent-hover text-nx-deep transition-all duration-150"
          >
            <Check size={14} strokeWidth={1.5} />
            Go to Plugins
          </button>
        </>
      )}
    </div>
  );
}

// ── Helpers ───────────────────────────────────────────────────────

function FieldInput({
  label,
  value,
  onChange,
  placeholder,
  mono,
  error,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  mono?: boolean;
  error?: string;
}) {
  return (
    <div>
      <label className="block text-[11px] font-medium text-nx-text-muted mb-1 uppercase tracking-wider">
        {label}
      </label>
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className={`w-full px-3 py-2 text-[13px] bg-nx-deep border border-nx-border-subtle rounded-[var(--radius-button)] text-nx-text placeholder:text-nx-text-ghost focus:outline-none focus:border-nx-accent transition-colors ${
          mono ? "font-mono" : ""
        } ${error ? "border-nx-error" : ""}`}
      />
      {error && (
        <p className="text-[10px] text-nx-error mt-0.5">{error}</p>
      )}
    </div>
  );
}
