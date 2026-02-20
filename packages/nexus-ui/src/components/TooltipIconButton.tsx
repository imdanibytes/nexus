import { forwardRef, type ComponentPropsWithRef } from "react";
import { Tooltip, Button } from "@heroui/react";
import { cn } from "../lib/utils";

export type TooltipIconButtonProps = ComponentPropsWithRef<typeof Button> & {
  tooltip: string;
  side?: "top" | "bottom" | "left" | "right";
};

export const TooltipIconButton = forwardRef<
  HTMLButtonElement,
  TooltipIconButtonProps
>(({ children, tooltip, side = "bottom", className, variant, ...rest }, ref) => {
  const placement = (side ?? "bottom") as "top" | "bottom" | "left" | "right";
  return (
    <Tooltip content={tooltip} placement={placement}>
      <Button
        variant={
          variant === "default"
            ? "solid"
            : variant === "outline"
              ? "bordered"
              : variant === "destructive"
                ? "solid"
                : "light"
        }
        color={variant === "default" ? "primary" : variant === "destructive" ? "danger" : "default"}
        isIconOnly
        size="sm"
        {...rest}
        className={cn("aui-button-icon size-6 min-w-6 p-1", className)}
        ref={ref}
      >
        {children}
        <span className="aui-sr-only sr-only">{tooltip}</span>
      </Button>
    </Tooltip>
  );
});

TooltipIconButton.displayName = "TooltipIconButton";
