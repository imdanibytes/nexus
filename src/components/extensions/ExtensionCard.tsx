import { useTranslation } from "react-i18next";
import type { ExtensionRegistryEntry } from "../../types/extension";
import { timeAgo } from "../../lib/timeAgo";
import { Card, CardBody, Chip } from "@heroui/react";

interface Props {
  entry: ExtensionRegistryEntry;
  onSelect: () => void;
}

export function ExtensionRegistryCard({ entry, onSelect }: Props) {
  const { t } = useTranslation("plugins");

  return (
    <Card
      isPressable
      onPress={onSelect}
      className="cursor-pointer transition-all duration-200"
    >
      <CardBody className="p-4">
      <div className="flex items-start justify-between mb-2">
        <div>
          <h3 className="text-[13px] font-semibold">
            {entry.name}
          </h3>
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
        {entry.status === "deprecated" && (
          <Chip size="sm" variant="flat" color="warning">
            {t("common:status.deprecated")}
          </Chip>
        )}
      </div>
      <p className="text-[11px] text-default-500 line-clamp-2">
        {entry.description}
      </p>
      <div className="flex items-center gap-1.5 mt-2.5 flex-wrap">
        {entry.source && (
          <Chip size="sm" variant="flat">
            {entry.source}
          </Chip>
        )}
        {entry.platforms?.map((platform) => (
          <Chip key={platform} size="sm" variant="flat">
            {t(`card.platforms.${platform}`, { defaultValue: platform })}
          </Chip>
        ))}
        {entry.categories.map((cat) => (
          <Chip key={cat} size="sm" variant="flat">
            {cat}
          </Chip>
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
