import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Search } from "lucide-react";
import { Input } from "@heroui/react";

interface Props {
  onSearch: (query: string) => void;
  initialQuery?: string;
}

export function SearchBar({ onSearch, initialQuery = "" }: Props) {
  const { t } = useTranslation("plugins");
  const [value, setValue] = useState(initialQuery);

  useEffect(() => {
    const timer = setTimeout(() => {
      onSearch(value);
    }, 300);
    return () => clearTimeout(timer);
  }, [value, onSearch]);

  return (
    <Input
      type="text"
      value={value}
      onValueChange={setValue}
      placeholder={t("marketplace.searchPlaceholder")}
      isClearable
      onClear={() => setValue("")}
      startContent={
        <Search
          size={15}
          strokeWidth={1.5}
          className="text-default-500"
        />
      }
    />
  );
}
