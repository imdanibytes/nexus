import { useTranslation } from "react-i18next";
import type { InstalledPlugin, RegistryEntry } from "../../types/plugin";
import type { PluginStatus } from "../../types/plugin";
import { timeAgo } from "../../lib/timeAgo";
import { Card, CardBody, Chip } from "@heroui/react";
import { HardDrive, Cloud } from "lucide-react";

const statusColor: Record<PluginStatus, "success" | "default" | "danger" | "warning"> = {
  running: "success",
  stopped: "default",
  error: "danger",
  installing: "warning",
};

interface InstalledPluginCardProps {
  plugin: InstalledPlugin;
  onSelect: () => void;
  isSelected: boolean;
}

export function InstalledPluginCard({
  plugin,
  onSelect,
  isSelected,
}: InstalledPluginCardProps) {
  const { t } = useTranslation("plugins");
  const color = statusColor[plugin.status];

  return (
    <Card
      isPressable
      onPress={onSelect}
    >
      <CardBody className="p-4">
      <div className="flex items-start justify-between mb-2">
        <div>
          <h3 className="text-[13px] font-semibold">
            {plugin.manifest.name}
          </h3>
          <p className="text-[11px] text-default-500 font-mono">v{plugin.manifest.version}</p>
        </div>
        <div className="flex items-center gap-1.5">
          {plugin.dev_mode && (
            <Chip size="sm" variant="flat" color="secondary">{t("common:status.dev")}</Chip>
          )}
          <Chip size="sm" variant="flat"
            startContent={plugin.local_manifest_path ? <HardDrive size={9} strokeWidth={1.5} /> : <Cloud size={9} strokeWidth={1.5} />}
          >
            {plugin.local_manifest_path ? t("common:status.local") : t("common:status.registry")}
          </Chip>
          <Chip size="sm" variant="flat" color={color}>{t(`common:status.${plugin.status}`)}</Chip>
        </div>
      </div>
      <p className="text-[11px] text-default-500 line-clamp-2">
        {plugin.manifest.description}
      </p>
      </CardBody>
    </Card>
  );
}

interface RegistryPluginCardProps {
  entry: RegistryEntry;
  onSelect: () => void;
  isInstalled: boolean;
}

export function RegistryPluginCard({
  entry,
  onSelect,
  isInstalled,
}: RegistryPluginCardProps) {
  const { t } = useTranslation("plugins");

  return (
    <Card
      isPressable
      onPress={onSelect}
    >
      <CardBody className="p-4">
      <div className="flex items-start justify-between mb-2">
        <div className="flex items-center gap-2.5">
          {entry.icon ? (
            <img
              src={entry.icon}
              alt=""
              className="w-8 h-8 rounded-[8px] object-cover flex-shrink-0"
            />
          ) : (
            <div className="w-8 h-8 rounded-[8px] bg-default-100 flex items-center justify-center flex-shrink-0">
              <span className="text-[13px] font-semibold text-default-500">
                {entry.name.charAt(0)}
              </span>
            </div>
          )}
          <div>
            <h3 className="text-[13px] font-semibold">{entry.name}</h3>
            <p className="text-[11px] text-default-500 font-mono">
              v{entry.version}
              {entry.author_url ? (
                <a
                  href={entry.author_url}
                  target="_blank"
                  rel="noopener noreferrer"
                  onClick={(e) => e.stopPropagation()}
                  className="font-sans ml-1.5 text-primary hover:underline"
                >
                  {entry.author}
                </a>
              ) : (
                <span className="font-sans ml-1.5">{t("common:by")} {entry.author}</span>
              )}
            </p>
          </div>
        </div>
        <div className="flex gap-1.5 flex-shrink-0">
          {entry.status === "deprecated" && (
            <Chip size="sm" variant="flat" color="warning">{t("common:status.deprecated")}</Chip>
          )}
          {isInstalled && (
            <Chip size="sm" variant="flat" color="secondary">{t("common:status.installed")}</Chip>
          )}
        </div>
      </div>
      <p className="text-[11px] text-default-500 line-clamp-2">
        {entry.description}
      </p>
      <div className="flex items-center gap-1.5 mt-2.5 flex-wrap">
        {entry.source && (
          <Chip size="sm" variant="flat" color="secondary">{entry.source}</Chip>
        )}
        {entry.categories.map((cat) => (
          <Chip key={cat} size="sm" variant="flat">{cat}</Chip>
        ))}
        {entry.created_at && (
          <span className="text-[10px] text-default-400 ml-auto">
            {timeAgo(entry.created_at)}
          </span>
        )}
      </div>
      </CardBody>
    </Card>
  );
}
