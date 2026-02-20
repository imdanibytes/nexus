import {
  useCallback,
  useRef,
  useState,
  type FC,
  type KeyboardEvent,
} from "react";
import { ArrowUpIcon, SquareIcon } from "lucide-react";
import { Button } from "@heroui/react";
import { TooltipIconButton } from "./TooltipIconButton";

export interface ComposerProps {
  onSend: (text: string) => void;
  onCancel: () => void;
  isStreaming: boolean;
  disabled?: boolean;
  placeholder?: string;
  leftSlot?: React.ReactNode;
}

export const Composer: FC<ComposerProps> = ({
  onSend,
  onCancel,
  isStreaming,
  disabled,
  placeholder = "Send a message...",
  leftSlot,
}) => {
  const [text, setText] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const handleSend = useCallback(() => {
    const trimmed = text.trim();
    if (!trimmed || isStreaming) return;
    onSend(trimmed);
    setText("");
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
    }
  }, [text, isStreaming, onSend]);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        handleSend();
      }
    },
    [handleSend],
  );

  const handleInput = useCallback(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  }, []);

  return (
    <div className="aui-composer-root relative flex w-full flex-col rounded-xl border border-default-200 dark:border-default-200/50 bg-default-100 dark:bg-default-100/80 backdrop-blur-2xl px-1 pt-2 outline-none transition-shadow has-[textarea:focus-visible]:border-primary/40 has-[textarea:focus-visible]:ring-2 has-[textarea:focus-visible]:ring-primary/20">
      <textarea
        ref={textareaRef}
        value={text}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={handleKeyDown}
        onInput={handleInput}
        placeholder={placeholder}
        disabled={disabled}
        className="aui-composer-input mb-1 max-h-32 min-h-14 w-full resize-none bg-transparent px-4 pt-2 pb-3 text-sm text-foreground outline-none placeholder:text-default-400 focus-visible:ring-0"
        rows={1}
        autoFocus
        aria-label="Message input"
      />
      <div className="aui-composer-action-wrapper relative mx-2 mb-2 flex items-center justify-between">
        <div className="flex items-center gap-1">
          {leftSlot}
        </div>
        {!isStreaming ? (
          <TooltipIconButton
            tooltip="Send message"
            side="bottom"
            variant="default"
            className="aui-composer-send size-8 min-w-8 rounded-full"
            aria-label="Send message"
            onPress={handleSend}
            isDisabled={!text.trim() || disabled}
          >
            <ArrowUpIcon className="aui-composer-send-icon size-4" />
          </TooltipIconButton>
        ) : (
          <Button
            color="primary"
            variant="solid"
            isIconOnly
            size="sm"
            className="aui-composer-cancel size-8 min-w-8 rounded-full"
            aria-label="Stop generating"
            onPress={onCancel}
          >
            <SquareIcon className="aui-composer-cancel-icon size-3 fill-current" />
          </Button>
        )}
      </div>
    </div>
  );
};
