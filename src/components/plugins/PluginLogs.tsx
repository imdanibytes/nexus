import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Copy, Check, Loader2, X } from "lucide-react";
import { Drawer, DrawerContent, DrawerHeader, DrawerBody } from "@heroui/react";
import { Button } from "@heroui/react";
import { Chip } from "@heroui/react";

interface Props {
  pluginId: string;
  getLogs: (pluginId: string, tail?: number) => Promise<string[]>;
  onClose: () => void;
}

export function PluginLogs({ pluginId, getLogs, onClose }: Props) {
  const { t } = useTranslation("plugins");
  const [logs, setLogs] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [copied, setCopied] = useState(false);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let active = true;

    async function fetchLogs() {
      const lines = await getLogs(pluginId, 200);
      if (active) {
        setLogs(lines);
        setLoading(false);
      }
    }

    fetchLogs();
    const interval = setInterval(fetchLogs, 3000);

    return () => {
      active = false;
      clearInterval(interval);
    };
  }, [pluginId, getLogs]);

  // Auto-scroll to bottom when new logs arrive
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  function handleCopy() {
    navigator.clipboard.writeText(logs.join("\n")).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  }

  return (
    <Drawer
      isOpen
      onOpenChange={(open) => { if (!open) onClose(); }}
      placement="bottom"
      hideCloseButton
    >
      <DrawerContent>
        <DrawerHeader className="px-4 py-2.5 border-b border-default-100 flex items-center gap-2 shrink-0">
          <div className="text-[12px] font-semibold text-default-500 flex items-center gap-2 flex-1">
            {t("logs.title")}
            <Chip size="sm" variant="flat">
              {pluginId}
            </Chip>
          </div>
          <Button
            onPress={handleCopy}
            isDisabled={logs.length === 0}
          >
            {copied ? (
              <Check size={12} strokeWidth={1.5} className="text-success" />
            ) : (
              <Copy size={12} strokeWidth={1.5} />
            )}
            {copied ? t("common:action.copied") : t("common:action.copy")}
          </Button>
          <Button
            isIconOnly
            onPress={onClose}
          >
            <X size={14} strokeWidth={1.5} />
          </Button>
        </DrawerHeader>

        <DrawerBody className="p-0 overflow-y-auto">
          <div className="p-4 font-mono text-[11px] leading-5">
            {loading && logs.length === 0 ? (
              <div className="flex items-center gap-2 text-default-400">
                <Loader2 size={14} strokeWidth={1.5} className="animate-spin" />
                {t("logs.loadingLogs")}
              </div>
            ) : logs.length === 0 ? (
              <p className="text-default-400">{t("logs.noLogs")}</p>
            ) : (
              logs.map((line, i) => (
                <div key={i} className="text-default-500 whitespace-pre-wrap hover:bg-default-100 px-1 -mx-1 rounded-sm">
                  {line}
                </div>
              ))
            )}
            <div ref={bottomRef} />
          </div>
        </DrawerBody>
      </DrawerContent>
    </Drawer>
  );
}
