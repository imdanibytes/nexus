import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useAppStore } from "../../stores/appStore";
import { oauthListClients, oauthRevokeClient } from "../../lib/tauri";
import type { OAuthClientInfo } from "../../types/oauth";
import { Shield, KeyRound, Search, ChevronDown, Trash2 } from "lucide-react";
import {
  Button,
  Input,
  Chip,
  Card,
  CardBody,
  Divider,
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
} from "@heroui/react";
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
    <Card>
      <CardBody className="p-5">
      <div className="flex items-center gap-2 mb-4">
        <KeyRound size={15} strokeWidth={1.5} className="text-default-500" />
        <h3 className="text-[14px] font-semibold">
          {t("securityTab.connectedClients")}
        </h3>
      </div>

      <p className="text-[11px] text-default-400 mb-4">
        {t("securityTab.connectedClientsDesc")}
      </p>

      {clients.length === 0 ? (
        <p className="text-[11px] text-default-400">
          {t("securityTab.noClients")}
        </p>
      ) : (
        <div className="space-y-2">
          {clients.map((client) => (
            <Card key={client.client_id}>
              <CardBody className="p-3 flex-row items-center justify-between">
              <div className="flex items-center gap-3 min-w-0">
                <span className="text-[13px] font-medium truncate">
                  {client.client_name}
                </span>
                <Chip
                  size="sm"
                  variant="flat"
                  color={client.approved ? "success" : "default"}
                >
                  {client.approved
                    ? t("securityTab.approved")
                    : t("securityTab.requiresConsent")}
                </Chip>
                <span className="text-[10px] text-default-400 font-mono flex-shrink-0">
                  {new Date(client.registered_at).toLocaleDateString()}
                </span>
              </div>
              <Button
                color="danger"
                onPress={() => setRevokeTarget(client)}
                startContent={<Trash2 size={10} strokeWidth={2} />}
              >
                {t("securityTab.revoke")}
              </Button>
              </CardBody>
            </Card>
          ))}
        </div>
      )}

      {/* Revoke confirmation dialog */}
      <Modal
        isOpen={revokeTarget !== null}
        onOpenChange={(open) => { if (!open) setRevokeTarget(null); }}
      >
        <ModalContent>
          {(onClose) => (
            <>
              <ModalHeader className="text-[14px]">
                {t("securityTab.revokeConfirm", {
                  clientName: revokeTarget?.client_name ?? "",
                })}
              </ModalHeader>
              <ModalBody>
                <p className="text-[12px] text-default-500">
                  {t("securityTab.revokeDetail")}
                </p>
              </ModalBody>
              <ModalFooter>
                <Button onPress={onClose}>{t("common:action.cancel")}</Button>
                <Button
                  color="danger"
                  onPress={() => revokeTarget && handleRevoke(revokeTarget.client_id)}
                >
                  {t("securityTab.revoke")}
                </Button>
              </ModalFooter>
            </>
          )}
        </ModalContent>
      </Modal>
    </CardBody>
    </Card>
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
      <Card>
        <CardBody className="p-5">
        <div className="flex items-center gap-2 mb-4">
          <Shield size={15} strokeWidth={1.5} className="text-default-500" />
          <h3 className="text-[14px] font-semibold">
            {t("pluginsTab.permissions")}
          </h3>
        </div>

        {installedPlugins.length === 0 ? (
          <p className="text-[11px] text-default-400">
            {t("pluginsTab.noPlugins")}
          </p>
        ) : (
          <>
            <div className="relative mb-4">
              <Input
                type="text"
                value={permSearch}
                onValueChange={setPermSearch}
                placeholder={t("pluginsTab.filterPlugins")}
                startContent={
                  <Search
                    size={14}
                    strokeWidth={1.5}
                    className="text-default-400"
                  />
                }
                variant="bordered"
              />
            </div>

            <div className="space-y-2">
              {filtered.length === 0 ? (
                <p className="text-[11px] text-default-400">
                  {t("pluginsTab.noMatch", { query: permSearch })}
                </p>
              ) : (
                filtered.map((plugin) => {
                  const id = plugin.manifest.id;
                  const isOpen = permExpanded.has(id);
                  const permCount = plugin.manifest.permissions.length;

                  return (
                    <Card key={id}>
                      <CardBody
                        as="button"
                        onClick={() => togglePerm(id)}
                        className="p-3 flex-row items-center justify-between cursor-pointer"
                      >
                        <div className="flex items-center gap-3 min-w-0">
                          <span className="text-[13px] font-medium truncate">
                            {plugin.manifest.name}
                          </span>
                          <span className="text-[11px] text-default-400 font-mono flex-shrink-0">
                            v{plugin.manifest.version}
                          </span>
                          <Chip
                            size="sm"
                            variant="flat"
                            color={plugin.status === "running" ? "success" : "default"}
                          >
                            {plugin.status}
                          </Chip>
                        </div>
                        <div className="flex items-center gap-2 flex-shrink-0 ml-2">
                          <span className="text-[11px] text-default-400">
                            {t("pluginsTab.permCount", { count: permCount })}
                          </span>
                          <ChevronDown
                            size={14}
                            strokeWidth={1.5}
                            className={`text-default-400 transition-transform duration-200 ${
                              isOpen ? "rotate-180" : ""
                            }`}
                          />
                        </div>
                      </CardBody>
                      {isOpen && (
                        <CardBody className="px-3 pb-3 pt-0">
                          <Divider className="mb-3" />
                          <PermissionList pluginId={id} />
                        </CardBody>
                      )}
                    </Card>
                  );
                })
              )}
            </div>
          </>
        )}
      </CardBody>
      </Card>
    </div>
  );
}
