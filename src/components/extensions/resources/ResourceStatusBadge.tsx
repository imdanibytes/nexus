import { Chip } from "@heroui/react";

const STATUS_COLOR: Record<string, "success" | "warning" | "danger" | "default"> = {
  active: "success",
  paused: "warning",
  error: "danger",
  completed: "default",
};

interface ResourceStatusBadgeProps {
  value: string;
}

export function ResourceStatusBadge({ value }: ResourceStatusBadgeProps) {
  const color = STATUS_COLOR[value.toLowerCase()] ?? "default";
  return (
    <Chip size="sm" variant="flat" color={color}>
      {value}
    </Chip>
  );
}
