import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useAppStore } from "../../stores/appStore";
import { oauthListClients, oauthRevokeClient } from "../../lib/tauri";
import type { OAuthClientInfo } from "../../types/oauth";
import { Shield, KeyRound, Search, ChevronDown, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Collapsible, CollapsibleTrigger, CollapsibleContent } from "@/components/ui/collapsible";
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogAction,
  AlertDialogCancel,
} from "@/components/ui/alert-dialog";
import { PermissionList } from "../permissions/PermissionList";

function ConnectedClients() {
  const { t } = useTranslation("settings");
  const [clients, setClients] = useState<OAuthClientInfo[]>([]);
  const [revokeTarget, setRevokeTarget] = useState<OAuthClientInfo | null>(null);

  const load = useCallback(() => {
    oauthListClients().then(setClients).catch(() => {});
  }, []);

  useEffect(() => {
    load();
    const id = setInterval(load, 3000);
    return () => clearInterval(id);
  }, [load]);

  async function handleRevoke(clientId: string) {
    try {
      await oauthRevokeClient(clientId);
      setRevokeTarget(null);
      load();
    } catch {
      /* ignore */
    }
  }

  return (
    <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
      <div className="flex items-center gap-2 mb-4">
        <KeyRound size={15} strokeWidth={1.5} className="text-nx-text-muted" />
        <h3 className="text-[14px] font-semibold text-nx-text">
          {t("securityTab.connectedClients")}
        </h3>
      </div>

      <p className="text-[11px] text-nx-text-ghost mb-4">
        {t("securityTab.connectedClientsDesc")}
      </p>

      {clients.length === 0 ? (
        <p className="text-[11px] text-nx-text-ghost">
          {t("securityTab.noClients")}
        </p>
      ) : (
        <div className="space-y-2">
          {clients.map((client) => (
            <div
              key={client.client_id}
              className="flex items-center justify-between p-3 rounded-[var(--radius-button)] border border-nx-border-subtle bg-nx-deep"
            >
              <div className="flex items-center gap-3 min-w-0">
                <span className="text-[13px] text-nx-text font-medium truncate">
                  {client.client_name}
                </span>
                <Badge
                  variant={client.approved ? "success" : "outline"}
                  className="text-[10px] flex-shrink-0"
                >
                  {client.approved
                    ? t("securityTab.approved")
                    : t("securityTab.requiresConsent")}
                </Badge>
                <span className="text-[10px] text-nx-text-ghost font-mono flex-shrink-0">
                  {new Date(client.registered_at).toLocaleDateString()}
                </span>
              </div>
              <Button
                variant="destructive"
                size="xs"
                onClick={() => setRevokeTarget(client)}
                className="flex-shrink-0 ml-2"
              >
                <Trash2 size={10} strokeWidth={2} />
                {t("securityTab.revoke")}
              </Button>
            </div>
          ))}
        </div>
      )}

      {/* Revoke confirmation dialog */}
      <AlertDialog
        open={revokeTarget !== null}
        onOpenChange={(open) => { if (!open) setRevokeTarget(null); }}
      >
        <AlertDialogContent className="max-w-sm">
          <AlertDialogHeader>
            <AlertDialogTitle className="text-[14px]">
              {t("securityTab.revokeConfirm", {
                clientName: revokeTarget?.client_name ?? "",
              })}
            </AlertDialogTitle>
            <AlertDialogDescription className="text-[12px]">
              {t("securityTab.revokeDetail")}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("common:action.cancel")}</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => revokeTarget && handleRevoke(revokeTarget.client_id)}
              className="bg-nx-error hover:bg-nx-error/80 text-white"
            >
              {t("securityTab.revoke")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </section>
  );
}

export function SecurityTab() {
  const { t } = useTranslation("settings");
  const { installedPlugins } = useAppStore();
  const [permSearch, setPermSearch] = useState("");
  const [permExpanded, setPermExpanded] = useState<Set<string>>(new Set());

  const filtered = installedPlugins.filter(
    (p) =>
      p.manifest.name.toLowerCase().includes(permSearch.toLowerCase()) ||
      p.manifest.id.toLowerCase().includes(permSearch.toLowerCase())
  );

  function togglePerm(id: string) {
    setPermExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  return (
    <div className="space-y-6">
      {/* Connected Clients */}
      <ConnectedClients />

      {/* Plugin Permissions */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center gap-2 mb-4">
          <Shield size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <h3 className="text-[14px] font-semibold text-nx-text">
            {t("pluginsTab.permissions")}
          </h3>
        </div>

        {installedPlugins.length === 0 ? (
          <p className="text-[11px] text-nx-text-ghost">
            {t("pluginsTab.noPlugins")}
          </p>
        ) : (
          <>
            <div className="relative mb-4">
              <Search
                size={14}
                strokeWidth={1.5}
                className="absolute left-3 top-1/2 -translate-y-1/2 text-nx-text-ghost"
              />
              <Input
                type="text"
                value={permSearch}
                onChange={(e) => setPermSearch(e.target.value)}
                placeholder={t("pluginsTab.filterPlugins")}
                className="pl-9"
              />
            </div>

            <div className="space-y-2">
              {filtered.length === 0 ? (
                <p className="text-[11px] text-nx-text-ghost">
                  {t("pluginsTab.noMatch", { query: permSearch })}
                </p>
              ) : (
                filtered.map((plugin) => {
                  const id = plugin.manifest.id;
                  const isOpen = permExpanded.has(id);
                  const permCount = plugin.manifest.permissions.length;

                  return (
                    <Collapsible key={id} open={isOpen} onOpenChange={() => togglePerm(id)}>
                      <div className="rounded-[var(--radius-button)] border border-nx-border-subtle bg-nx-deep overflow-hidden">
                        <CollapsibleTrigger asChild>
                          <button className="w-full flex items-center justify-between p-3 hover:bg-nx-wash/30 transition-colors duration-150">
                            <div className="flex items-center gap-3 min-w-0">
                              <span className="text-[13px] text-nx-text font-medium truncate">
                                {plugin.manifest.name}
                              </span>
                              <span className="text-[11px] text-nx-text-ghost font-mono flex-shrink-0">
                                v{plugin.manifest.version}
                              </span>
                              <Badge
                                variant={plugin.status === "running" ? "success" : "secondary"}
                                className="text-[10px]"
                              >
                                {plugin.status}
                              </Badge>
                            </div>
                            <div className="flex items-center gap-2 flex-shrink-0 ml-2">
                              <span className="text-[11px] text-nx-text-ghost">
                                {t("pluginsTab.permCount", { count: permCount })}
                              </span>
                              <ChevronDown
                                size={14}
                                strokeWidth={1.5}
                                className={`text-nx-text-ghost transition-transform duration-200 ${
                                  isOpen ? "rotate-180" : ""
                                }`}
                              />
                            </div>
                          </button>
                        </CollapsibleTrigger>
                        <CollapsibleContent>
                          <div className="px-3 pb-3 border-t border-nx-border-subtle">
                            <div className="pt-3">
                              <PermissionList pluginId={id} />
                            </div>
                          </div>
                        </CollapsibleContent>
                      </div>
                    </Collapsible>
                  );
                })
              )}
            </div>
          </>
        )}
      </section>
    </div>
  );
}
