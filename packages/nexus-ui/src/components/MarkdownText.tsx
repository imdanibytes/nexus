import { memo, useCallback, type FC } from "react";
import { Streamdown, type Components } from "streamdown";
import { createMathPlugin } from "@streamdown/math";
import { createCodePlugin } from "@streamdown/code";
import remarkGfm from "remark-gfm";
import { remarkHighlight, remarkSubSuperscript, remarkAbbreviations } from "../lib/remark-plugins";
import "katex/dist/katex.min.css";
import { Tooltip } from "@heroui/react";
import { cn } from "../lib/utils";

interface MarkdownTextProps {
  text: string;
  isStreaming?: boolean;
}

// Stable plugin instances — created once, not per render
const streamdownPlugins = {
  math: createMathPlugin({ singleDollarTextMath: true }),
  code: createCodePlugin({ themes: ["github-light", "github-dark"] }),
};

const streamdownRemarkPlugins: import("streamdown").StreamdownProps["remarkPlugins"] = [
  [remarkGfm, { singleTilde: false }],
  remarkHighlight,
  remarkSubSuperscript,
  remarkAbbreviations,
];

const MarkdownTextImpl: FC<MarkdownTextProps> = ({ text, isStreaming }) => {
  return (
    <Streamdown
      mode={isStreaming ? "streaming" : "static"}
      isAnimating={isStreaming}
      components={markdownComponents}
      plugins={streamdownPlugins}
      remarkPlugins={streamdownRemarkPlugins}
      allowedTags={{
        mark: ["className", "style"],
        sub: [],
        sup: [],
        abbr: ["title"],
        section: ["dataFootnotes"],
      }}
      className="aui-md"
    >
      {text}
    </Streamdown>
  );
};

export const MarkdownText = memo(MarkdownTextImpl);

// ── Component overrides ──

const markdownComponents: Components = {
  h1: ({ className, node: _, ...props }) => (
    <h1
      className={cn(
        "aui-md-h1 mb-2 scroll-m-20 font-semibold text-base first:mt-0 last:mb-0",
        className,
      )}
      {...props}
    />
  ),
  h2: ({ className, node: _, ...props }) => (
    <h2
      className={cn(
        "aui-md-h2 mt-3 mb-1.5 scroll-m-20 font-semibold text-sm first:mt-0 last:mb-0",
        className,
      )}
      {...props}
    />
  ),
  h3: ({ className, node: _, ...props }) => (
    <h3
      className={cn(
        "aui-md-h3 mt-2.5 mb-1 scroll-m-20 font-semibold text-sm first:mt-0 last:mb-0",
        className,
      )}
      {...props}
    />
  ),
  h4: ({ className, node: _, ...props }) => (
    <h4
      className={cn(
        "aui-md-h4 mt-2 mb-1 scroll-m-20 font-medium text-sm first:mt-0 last:mb-0",
        className,
      )}
      {...props}
    />
  ),
  h5: ({ className, node: _, ...props }) => (
    <h5
      className={cn(
        "aui-md-h5 mt-2 mb-1 font-medium text-sm first:mt-0 last:mb-0",
        className,
      )}
      {...props}
    />
  ),
  h6: ({ className, node: _, ...props }) => (
    <h6
      className={cn(
        "aui-md-h6 mt-2 mb-1 font-medium text-sm first:mt-0 last:mb-0",
        className,
      )}
      {...props}
    />
  ),
  p: ({ className, node: _, ...props }) => (
    <p
      className={cn(
        "aui-md-p my-2.5 leading-normal first:mt-0 last:mb-0",
        className,
      )}
      {...props}
    />
  ),
  a: ({ className, node: _, href, ...props }) => {
    const isAnchor = href?.startsWith("#");

    const handleClick = useCallback(
      (e: React.MouseEvent<HTMLAnchorElement>) => {
        if (!isAnchor || !href) return;
        e.preventDefault();
        const id = href.slice(1);
        const el =
          document.getElementById(id) ||
          document.getElementById(`user-content-${id}`);
        if (!el) return;
        el.scrollIntoView({ behavior: "smooth", block: "center" });
        el.classList.add("aui-md-flash");
        setTimeout(() => el.classList.remove("aui-md-flash"), 1500);
      },
      [isAnchor, href],
    );

    return (
      <a
        className={cn(
          "aui-md-a text-primary underline underline-offset-2 hover:text-primary/80 cursor-pointer",
          isAnchor && "no-underline",
          className,
        )}
        href={href}
        onClick={isAnchor ? handleClick : undefined}
        {...(!isAnchor && { target: "_blank", rel: "noopener noreferrer" })}
        {...props}
      />
    );
  },
  blockquote: ({ className, node: _, ...props }) => (
    <blockquote
      className={cn(
        "aui-md-blockquote my-2.5 border-default-400/30 border-l-2 pl-3 text-default-500 italic",
        className,
      )}
      {...props}
    />
  ),
  ul: ({ className, node: _, ...props }) => (
    <ul
      className={cn(
        "aui-md-ul my-2 ml-4 list-disc marker:text-default-500 [&>li]:mt-1",
        className,
      )}
      {...props}
    />
  ),
  ol: ({ className, node: _, ...props }) => (
    <ol
      className={cn(
        "aui-md-ol my-2 ml-4 list-decimal marker:text-default-500 [&>li]:mt-1",
        className,
      )}
      {...props}
    />
  ),
  li: ({ className, node: _, ...props }) => (
    <li className={cn("aui-md-li leading-normal", className)} {...props} />
  ),
  hr: ({ className, node: _, ...props }) => (
    <hr
      className={cn("aui-md-hr my-2 border-default-400/20", className)}
      {...props}
    />
  ),
  table: ({ className, node: _, ...props }) => (
    <table
      className={cn(
        "aui-md-table my-2 w-full border-separate border-spacing-0 overflow-y-auto",
        className,
      )}
      {...props}
    />
  ),
  th: ({ className, node: _, ...props }) => (
    <th
      className={cn(
        "aui-md-th bg-default-100/40 px-2 py-1 text-left font-medium first:rounded-tl-lg last:rounded-tr-lg",
        className,
      )}
      {...props}
    />
  ),
  td: ({ className, node: _, ...props }) => (
    <td
      className={cn(
        "aui-md-td border-default-400/20 border-b border-l px-2 py-1 text-left last:border-r [[align=center]]:text-center [[align=right]]:text-right",
        className,
      )}
      {...props}
    />
  ),
  tr: ({ className, node: _, ...props }) => (
    <tr
      className={cn(
        "aui-md-tr m-0 border-b p-0 first:border-t [&:last-child>td:first-child]:rounded-bl-lg [&:last-child>td:last-child]:rounded-br-lg",
        className,
      )}
      {...props}
    />
  ),
  mark: ({ className, node: _, ...props }) => (
    <mark
      className={cn("aui-md-mark rounded-sm px-0.5 text-inherit", className)}
      style={{ backgroundColor: "hsl(var(--heroui-primary) / 0.2)" }}
      {...props}
    />
  ),
  abbr: ({ className, node: _, title, children, ...props }) => (
    <Tooltip content={title} placement="top" className="max-w-xs text-xs">
      <abbr
        className={cn(
          "aui-md-abbr cursor-help border-b border-dotted border-default-400/50 no-underline",
          className,
        )}
        title={undefined}
        {...props}
      >
        {children}
      </abbr>
    </Tooltip>
  ),
  section: ({ className, node: _, ...props }) => {
    const isFootnotes =
      (props as Record<string, unknown>)["data-footnotes"] !== undefined;
    return (
      <section
        className={cn(
          isFootnotes &&
            "aui-md-footnotes mt-4 border-t border-default-400/20 pt-3 text-xs text-default-500 [&_ol]:ml-4 [&_ol]:list-decimal [&_li]:mt-1",
          className,
        )}
        {...props}
      />
    );
  },
  sub: ({ className, node: _, ...props }) => (
    <sub
      className={cn("aui-md-sub", className)}
      {...props}
    />
  ),
  sup: ({ className, node: _, ...props }) => (
    <sup
      className={cn(
        "aui-md-sup [&>a]:text-xs [&>a]:no-underline",
        className,
      )}
      {...props}
    />
  ),
};
