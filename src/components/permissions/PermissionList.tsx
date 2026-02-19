import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { usePermissions } from "../../hooks/usePermissions";
import { getPermissionInfo } from "../../types/permissions";
import type { Permission, GrantedPermission } from "../../types/permissions";
import { ChevronDown, FolderOpen, RotateCcw, X, ShieldCheck, Clock } from "lucide-react";
import { Button, Modal, ModalContent, ModalHeader, ModalBody, ModalFooter, Chip } from "@heroui/react";

interface Props {
  pluginId: string;
}

export function PermissionList({ pluginId }: Props) {
  const { t } = useTranslation("permissions");
  const { grants, loadGrants, revoke, unrevoke, removePath } = usePermissions();
  const [expandedPerms, setExpandedPerms] = useState<Set<string>>(new Set());
  const [confirmRestore, setConfirmRestore] = useState<string | null>(null);

  useEffect(() => {
    loadGrants(pluginId);
  }, [pluginId, loadGrants]);

  function togglePerm(perm: string) {
    setExpandedPerms((prev) => {
      const next = new Set(prev);
      if (next.has(perm)) next.delete(perm);
      else next.add(perm);
      return next;
    });
  }

  if (grants.length === 0) {
    return (
      <p className="text-[11px] text-default-400">{t("list.noPermissions")}</p>
    );
  }

  const activeGrants = grants.filter((g) => g.state === "active");
  const deferredGrants = grants.filter((g) => g.state === "deferred");
  const revokedGrants = grants.filter((g) => g.state === "revoked");

  const FS_PERMISSIONS = ["filesystem:read", "filesystem:write"];

  return (
    <div className="space-y-1.5">
      {/* Active permissions */}
      {activeGrants.map((grant) => (
        <ActivePermissionRow
          key={grant.permission}
          grant={grant}
          pluginId={pluginId}
          fsPermissions={FS_PERMISSIONS}
          expandedPerms={expandedPerms}
          onToggle={togglePerm}
          onRevoke={revoke}
          onRemovePath={removePath}
        />
      ))}

      {/* Deferred permissions */}
      {deferredGrants.length > 0 && (
        <>
          {activeGrants.length > 0 && (
            <div className="flex items-center gap-2 pt-2 pb-0.5">
              <div className="flex-1 h-px bg-default-100" />
              <span className="text-[10px] text-warning font-medium uppercase tracking-wide">
                {t("list.deferred")}
              </span>
              <div className="flex-1 h-px bg-default-100" />
            </div>
          )}

          {deferredGrants.map((grant) => {
            const info = getPermissionInfo(grant.permission);

            return (
              <div
                key={`deferred-${grant.permission}`}
                className="rounded-[8px] border border-warning/20 bg-background overflow-hidden"
              >
                <div className="flex items-center justify-between p-2.5">
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <p className="text-[12px] font-medium font-mono">
                        {grant.permission}
                      </p>
                      <Chip size="sm" variant="flat" color="warning" startContent={<Clock size={10} strokeWidth={1.5} />}>
                        {t("common:status.deferred")}
                      </Chip>
                    </div>
                    <p className="text-[11px] text-default-500 mt-0.5">
                      {info?.description ?? t("permissions:meta.unknown")}
                    </p>
                    <p className="text-[10px] text-default-400 mt-0.5">
                      {t("list.willPrompt")}
                    </p>
                  </div>
                  <div className="flex gap-1.5 flex-shrink-0 ml-2">
                    <Button
                      color="primary"
                      onPress={() => unrevoke(pluginId, [grant.permission as Permission])}
                      title={t("list.activateTooltip")}
                    >
                      <ShieldCheck size={11} strokeWidth={1.5} />
                      {t("list.activate")}
                    </Button>
                    <Button
                      color="danger"
                      onPress={() => revoke(pluginId, [grant.permission as Permission])}
                    >
                      {t("list.revoke")}
                    </Button>
                  </div>
                </div>
              </div>
            );
          })}
        </>
      )}

      {/* Revoked permissions */}
      {revokedGrants.length > 0 && (
        <>
          {(activeGrants.length > 0 || deferredGrants.length > 0) && (
            <div className="flex items-center gap-2 pt-2 pb-0.5">
              <div className="flex-1 h-px bg-default-100" />
              <span className="text-[10px] text-default-400 font-medium uppercase tracking-wide">
                {t("list.revoked")}
              </span>
              <div className="flex-1 h-px bg-default-100" />
            </div>
          )}

          {revokedGrants.map((grant) => {
            const info = getPermissionInfo(grant.permission);
            const scopeCount = grant.approved_scopes?.length ?? 0;

            return (
              <div
                key={`revoked-${grant.permission}`}
                className="rounded-[8px] border border-default-100 bg-background overflow-hidden opacity-60"
              >
                <div className="flex items-center justify-between p-2.5">
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <p className="text-[12px] text-default-500 font-medium font-mono line-through">
                        {grant.permission}
                      </p>
                      <Chip size="sm" variant="flat" color="danger">
                        {t("common:status.revoked")}
                      </Chip>
                      {scopeCount > 0 && (
                        <span className="text-[10px] text-default-400 font-mono flex-shrink-0">
                          {t("list.savedScopes", { count: scopeCount })}
                        </span>
                      )}
                    </div>
                    <p className="text-[11px] text-default-400 mt-0.5">
                      {info?.description ?? t("permissions:meta.unknown")}
                    </p>
                  </div>
                  <Button
                    color="primary"
                    onPress={() => setConfirmRestore(grant.permission)}
                    className="flex-shrink-0 ml-2"
                  >
                    <RotateCcw size={11} strokeWidth={1.5} />
                    {t("list.restore")}
                  </Button>
                </div>
              </div>
            );
          })}
        </>
      )}

      {/* Confirm restore modal */}
      <Modal
        isOpen={confirmRestore !== null}
        onOpenChange={(open) => { if (!open) setConfirmRestore(null); }}
      >
        <ModalContent>
          {(onClose) => (
            <>
              <ModalHeader className="text-[14px]">
                {t("list.restorePermission")}
              </ModalHeader>
              <ModalBody>
                <p className="text-[12px] text-default-500 mb-1">
                  Restore <span className="font-mono font-medium">{confirmRestore}</span> for this plugin?
                </p>
                <p className="text-[11px] text-default-400">
                  {t("list.restoreDetail")}
                </p>
              </ModalBody>
              <ModalFooter>
                <Button onPress={onClose}>
                  {t("common:action.cancel")}
                </Button>
                <Button
                  color="primary"
                  onPress={() => {
                    unrevoke(pluginId, [confirmRestore as Permission]);
                    setConfirmRestore(null);
                  }}
                >
                  {t("list.restore")}
                </Button>
              </ModalFooter>
            </>
          )}
        </ModalContent>
      </Modal>
    </div>
  );
}

/** Active permission row with scope expansion support. */
function ActivePermissionRow({
  grant,
  pluginId,
  fsPermissions,
  expandedPerms,
  onToggle,
  onRevoke,
  onRemovePath,
}: {
  grant: GrantedPermission;
  pluginId: string;
  fsPermissions: string[];
  expandedPerms: Set<string>;
  onToggle: (perm: string) => void;
  onRevoke: (pluginId: string, permissions: Permission[]) => void;
  onRemovePath: (pluginId: string, permission: Permission, path: string) => void;
}) {
  const { t } = useTranslation("permissions");
  const info = getPermissionInfo(grant.permission);
  const isFs = fsPermissions.includes(grant.permission);
  const hasPaths =
    isFs &&
    grant.approved_scopes !== null &&
    grant.approved_scopes !== undefined;
  const paths = grant.approved_scopes ?? [];
  const isExpanded = expandedPerms.has(grant.permission);

  return (
    <div className="rounded-[8px] border border-default-100 bg-background overflow-hidden">
      {/* Permission row */}
      <div
        onClick={hasPaths ? () => onToggle(grant.permission) : undefined}
        className={`flex items-center justify-between p-2.5 ${hasPaths ? "cursor-pointer hover:bg-default-200/30 transition-colors duration-150" : ""}`}
      >
        <div className="flex items-center gap-2 min-w-0">
          {hasPaths && (
            <ChevronDown
              size={14}
              strokeWidth={1.5}
              className={`text-default-500 flex-shrink-0 transition-transform duration-200 ${
                isExpanded ? "rotate-0" : "-rotate-90"
              }`}
            />
          )}
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <p className="text-[12px] font-medium font-mono">
                {grant.permission}
              </p>
              {hasPaths && (
                <span className="text-[10px] text-default-400 font-mono flex-shrink-0">
                  {paths.length === 0
                    ? t("list.noPathsApproved")
                    : t("list.pathCount", { count: paths.length })}
                </span>
              )}
              {isFs && grant.approved_scopes === null && (
                <Chip size="sm" variant="flat" color="warning">
                  {t("common:status.unrestricted")}
                </Chip>
              )}
            </div>
            <p className="text-[11px] text-default-500 mt-0.5">
              {info?.description ?? t("permissions:meta.unknown")}
            </p>
          </div>
        </div>
        <Button
          color="danger"
          onPress={(e) => {
            onRevoke(pluginId, [grant.permission as Permission]);
          }}
          className="flex-shrink-0 ml-2"
        >
          {t("list.revoke")}
        </Button>
      </div>

      {/* Approved paths (expanded) */}
      {hasPaths && isExpanded && (
        <div className="px-2.5 pb-2.5 border-t border-default-100">
          {paths.length === 0 ? (
            <p className="text-[11px] text-default-400 pt-2">
              {t("list.noDirectoriesApproved")}
            </p>
          ) : (
            <div className="pt-2 space-y-1">
              {paths.map((p) => (
                <div
                  key={p}
                  className="flex items-center justify-between gap-2 px-2 py-1.5 rounded-[6px] bg-default-100"
                >
                  <div className="flex items-center gap-2 min-w-0">
                    <FolderOpen
                      size={12}
                      strokeWidth={1.5}
                      className="text-primary flex-shrink-0"
                    />
                    <span className="text-[11px] font-mono truncate">
                      {p}
                    </span>
                  </div>
                  <Button
                    isIconOnly
                    onPress={() =>
                      onRemovePath(
                        pluginId,
                        grant.permission as Permission,
                        p
                      )
                    }
                    title={t("list.revokeAccessTo", { path: p })}
                    className="flex-shrink-0"
                  >
                    <X size={12} strokeWidth={1.5} />
                  </Button>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
