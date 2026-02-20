import { useTranslation } from "react-i18next";
import { Search } from "lucide-react";
import { SearchBar as NxSearchBar } from "@imdanibytes/nexus-ui";

interface Props {
  onSearch: (query: string) => void;
  initialQuery?: string;
}

export function SearchBar({ onSearch, initialQuery = "" }: Props) {
  const { t } = useTranslation("plugins");

  return (
    <NxSearchBar
      value={initialQuery}
      onChange={onSearch}
      placeholder={t("marketplace.searchPlaceholder")}
      icon={
        <Search
          size={15}
          strokeWidth={1.5}
          className="text-default-500"
        />
      }
    />
  );
}
