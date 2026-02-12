import { useEffect, useState } from "react";
import { Search, X } from "lucide-react";

interface Props {
  onSearch: (query: string) => void;
  initialQuery?: string;
}

export function SearchBar({ onSearch, initialQuery = "" }: Props) {
  const [value, setValue] = useState(initialQuery);

  useEffect(() => {
    const timer = setTimeout(() => {
      onSearch(value);
    }, 300);
    return () => clearTimeout(timer);
  }, [value, onSearch]);

  return (
    <div className="relative">
      <Search
        size={15}
        strokeWidth={1.5}
        className="absolute left-3 top-1/2 -translate-y-1/2 text-nx-text-muted"
      />
      <input
        type="text"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        placeholder="Search plugins..."
        className="w-full pl-10 pr-10 py-2.5 bg-nx-wash border border-nx-border-strong rounded-[var(--radius-input)] text-[13px] text-nx-text placeholder:text-nx-text-muted focus:outline-none focus:shadow-[var(--shadow-focus)] transition-shadow duration-150"
      />
      {value && (
        <button
          onClick={() => setValue("")}
          className="absolute right-3 top-1/2 -translate-y-1/2 text-nx-text-muted hover:text-nx-text transition-colors duration-150"
        >
          <X size={14} strokeWidth={1.5} />
        </button>
      )}
    </div>
  );
}
