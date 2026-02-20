/**
 * Custom remark transformer plugins for extended markdown syntax.
 *
 * These run AFTER remark-gfm (configured with singleTilde: false) so:
 *  - Footnote references [^1] are already parsed → no conflict with ^superscript^
 *  - Single tildes are not consumed as strikethrough → free for ~subscript~
 */

import type { Root, PhrasingContent, RootContent } from "mdast";
import { visit } from "unist-util-visit";

// ── Mark / Highlight ──

const MARK_RE = /==([^\n]+?)==/g;

/**
 * Transforms ==text== into <mark>text</mark>.
 *
 * Uses the same tree-transformer approach as the sub/sup plugin instead
 * of a micromark extension, which avoids compatibility issues with
 * streamdown's bundled parser.
 */
export function remarkHighlight() {
  return (tree: Root) => {
    visit(tree, "text", (node, index, parent) => {
      if (index === undefined || !parent) return;

      MARK_RE.lastIndex = 0;
      const parts: PhrasingContent[] = [];
      let last = 0;
      let match: RegExpExecArray | null;

      while ((match = MARK_RE.exec(node.value)) !== null) {
        if (match.index > last) {
          parts.push({ type: "text", value: node.value.slice(last, match.index) });
        }
        parts.push({ type: "html", value: `<mark>${match[1]}</mark>` });
        last = match.index + match[0].length;
      }

      if (parts.length === 0) return;

      if (last < node.value.length) {
        parts.push({ type: "text", value: node.value.slice(last) });
      }

      parent.children.splice(index, 1, ...(parts as RootContent[]));
      return index + parts.length;
    });
  };
}

// ── Subscript & Superscript ──

const SUB_SUPER_RE = /~([^~\s][^~]*)~|\^([^^\s][^^]*)\^/g;

/**
 * Transforms ~text~ into <sub> and ^text^ into <sup>.
 *
 * Requires remark-gfm configured with `{ singleTilde: false }` so that
 * `~text~` isn't eaten as strikethrough before we see it.
 */
export function remarkSubSuperscript() {
  return (tree: Root) => {
    visit(tree, "text", (node, index, parent) => {
      if (index === undefined || !parent) return;

      SUB_SUPER_RE.lastIndex = 0;
      const parts: PhrasingContent[] = [];
      let last = 0;
      let match: RegExpExecArray | null;

      while ((match = SUB_SUPER_RE.exec(node.value)) !== null) {
        if (match.index > last) {
          parts.push({ type: "text", value: node.value.slice(last, match.index) });
        }

        if (match[1] !== undefined) {
          parts.push({ type: "html", value: `<sub>${match[1]}</sub>` });
        } else if (match[2] !== undefined) {
          parts.push({ type: "html", value: `<sup>${match[2]}</sup>` });
        }

        last = match.index + match[0].length;
      }

      if (parts.length === 0) return;

      if (last < node.value.length) {
        parts.push({ type: "text", value: node.value.slice(last) });
      }

      parent.children.splice(index, 1, ...(parts as RootContent[]));
      // Return the new index so visit skips the nodes we just inserted
      return index + parts.length;
    });
  };
}

// ── Abbreviations ──

const ABBR_DEF_RE = /^\*\[([^\]]+)\]:\s*(.+)$/gm;

/**
 * Transforms *[ABBR]: Full text definitions into <abbr> tags.
 *
 * Definitions are removed from the document. All occurrences of the
 * abbreviated term in text nodes are wrapped in <abbr title="...">.
 */
export function remarkAbbreviations() {
  return (tree: Root) => {
    // 1. Collect abbreviation definitions from paragraph text
    const abbrs = new Map<string, string>();

    visit(tree, "text", (node) => {
      ABBR_DEF_RE.lastIndex = 0;
      let match: RegExpExecArray | null;
      while ((match = ABBR_DEF_RE.exec(node.value)) !== null) {
        abbrs.set(match[1], match[2]);
      }
    });

    if (abbrs.size === 0) return;

    // 2. Remove definition paragraphs (paragraphs whose only content is definitions)
    visit(tree, "paragraph", (node, index, parent) => {
      if (index === undefined || !parent) return;

      const text = node.children
        .filter((c): c is Extract<typeof c, { type: "text" }> => c.type === "text")
        .map((c) => c.value)
        .join("");

      const stripped = text.replace(ABBR_DEF_RE, "").trim();
      if (stripped === "") {
        parent.children.splice(index, 1);
        return index; // Re-visit same index since we removed
      }
    });

    // 3. Build a regex that matches any abbreviation (longest first to avoid partial matches)
    const sorted = [...abbrs.keys()].sort((a, b) => b.length - a.length);
    const abbrRe = new RegExp(`\\b(${sorted.map(escapeRegex).join("|")})\\b`, "g");

    // 4. Replace occurrences in text nodes
    visit(tree, "text", (node, index, parent) => {
      if (index === undefined || !parent) return;

      abbrRe.lastIndex = 0;
      const parts: PhrasingContent[] = [];
      let last = 0;
      let match: RegExpExecArray | null;

      while ((match = abbrRe.exec(node.value)) !== null) {
        if (match.index > last) {
          parts.push({ type: "text", value: node.value.slice(last, match.index) });
        }

        const title = abbrs.get(match[1])!;
        parts.push({
          type: "html",
          value: `<abbr title="${escapeHtml(title)}">${match[1]}</abbr>`,
        });

        last = match.index + match[0].length;
      }

      if (parts.length === 0) return;

      if (last < node.value.length) {
        parts.push({ type: "text", value: node.value.slice(last) });
      }

      parent.children.splice(index, 1, ...(parts as RootContent[]));
      return index + parts.length;
    });
  };
}

function escapeRegex(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}
