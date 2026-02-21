import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Button,
  Input,
  Select,
  SelectItem,
  Textarea,
  Switch,
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
} from "@heroui/react";
import type { ResourceTypeDef } from "../../../types/extension";

interface ResourceFormProps {
  isOpen: boolean;
  mode: "create" | "edit";
  typeDef: ResourceTypeDef;
  initialValues?: Record<string, unknown>;
  onClose: () => void;
  onSubmit: (data: Record<string, unknown>) => Promise<void>;
}

interface SchemaProperty {
  type?: string;
  enum?: string[];
  "x-resource-role"?: string;
  "x-display"?: {
    input?: string;
    variant?: string;
    label?: string;
  };
  title?: string;
  description?: string;
}

function renderField(
  key: string,
  prop: SchemaProperty,
  value: unknown,
  onChange: (key: string, val: unknown) => void
) {
  const label = prop["x-display"]?.label ?? prop.title ?? key;
  const display = prop["x-display"];

  if (prop.type === "boolean") {
    return (
      <div key={key} className="flex items-center justify-between py-1">
        <span className="text-[12px] text-default-500">{label}</span>
        <Switch
          size="sm"
          isSelected={!!value}
          // eslint-disable-next-line react-perf/jsx-no-new-function-as-prop
          onValueChange={(checked) => onChange(key, checked)}
        />
      </div>
    );
  }

  if (prop.type === "string" && prop.enum) {
    return (
      <Select
        key={key}
        label={label}
        size="sm"
        // eslint-disable-next-line react-perf/jsx-no-new-array-as-prop
        selectedKeys={value ? [String(value)] : []}
        // eslint-disable-next-line react-perf/jsx-no-new-function-as-prop
        onSelectionChange={(keys) => {
          const selected = Array.from(keys)[0];
          if (selected) onChange(key, String(selected));
        }}
        // eslint-disable-next-line react-perf/jsx-no-new-object-as-prop
        classNames={{ label: "text-[11px]", value: "text-[12px]" }}
      >
        {prop.enum.map((opt) => (
          <SelectItem key={opt}>{opt}</SelectItem>
        ))}
      </Select>
    );
  }

  if (prop.type === "object") {
    const textVal = typeof value === "object" && value !== null
      ? JSON.stringify(value, null, 2)
      : String(value ?? "");
    return (
      <Textarea
        key={key}
        label={label}
        size="sm"
        value={textVal}
        // eslint-disable-next-line react-perf/jsx-no-new-function-as-prop
        onValueChange={(v) => {
          try {
            onChange(key, JSON.parse(v));
          } catch {
            onChange(key, v);
          }
        }}
        // eslint-disable-next-line react-perf/jsx-no-new-object-as-prop
        classNames={{ label: "text-[11px]", input: "text-[12px] font-mono" }}
      />
    );
  }

  const inputType =
    display?.input === "password" ? "password"
    : prop.type === "integer" ? "number"
    : "text";

  return (
    <Input
      key={key}
      label={label}
      size="sm"
      type={inputType}
      value={String(value ?? "")}
      // eslint-disable-next-line react-perf/jsx-no-new-function-as-prop
      onValueChange={(v) => onChange(key, prop.type === "integer" ? Number(v) : v)}
      // eslint-disable-next-line react-perf/jsx-no-new-object-as-prop
      classNames={{ label: "text-[11px]", input: "text-[12px]" }}
    />
  );
}

export function ResourceForm({
  isOpen,
  mode,
  typeDef,
  initialValues,
  onClose,
  onSubmit,
}: ResourceFormProps) {
  const { t } = useTranslation("settings");
  const [values, setValues] = useState<Record<string, unknown>>(initialValues ?? {});
  const [submitting, setSubmitting] = useState(false);

  const schema = typeDef.schema as { properties?: Record<string, SchemaProperty> };
  const properties = schema.properties ?? {};

  const editableFields = Object.entries(properties).filter(
    ([, prop]) => prop["x-resource-role"] === "editable"
  );

  function handleChange(key: string, val: unknown) {
    setValues((prev) => ({ ...prev, [key]: val }));
  }

  async function handleSubmit() {
    setSubmitting(true);
    try {
      await onSubmit(values);
      onClose();
    } finally {
      setSubmitting(false);
    }
  }

  const title = mode === "create"
    ? t("extensionsTab.resourceCreate", { label: typeDef.label })
    : t("extensionsTab.resourceEdit", { label: typeDef.label });

  const handleOpenChange = useCallback(
    (open: boolean) => { if (!open) onClose(); },
    [onClose]
  );

  const handleCancelPress = useCallback(
    (onModalClose: () => void) => () => { onModalClose(); onClose(); },
    [onClose]
  );

  return (
    <Modal isOpen={isOpen} onOpenChange={handleOpenChange}>
      <ModalContent>
        {(onModalClose) => (
          <>
            <ModalHeader className="text-[14px]">{title}</ModalHeader>
            <ModalBody>
              <div className="space-y-3">
                {editableFields.map(([key, prop]) =>
                  renderField(key, prop, values[key], handleChange)
                )}
                {editableFields.length === 0 && (
                  <p className="text-[12px] text-default-400">
                    No editable fields defined.
                  </p>
                )}
              </div>
            </ModalBody>
            <ModalFooter>
              <Button variant="flat" onPress={handleCancelPress(onModalClose)}>
                {t("extensionsTab.resourceCancel")}
              </Button>
              <Button
                color="primary"
                isLoading={submitting}
                onPress={handleSubmit}
              >
                {t("extensionsTab.resourceSave")}
              </Button>
            </ModalFooter>
          </>
        )}
      </ModalContent>
    </Modal>
  );
}
