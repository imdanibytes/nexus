# Nexus Visual Language

This is the authoritative spec for how Nexus and its plugins look. If you're building a plugin UI that renders inside a Nexus viewport iframe, follow this document exactly.

## Stack

| Layer | What | Package |
|-------|------|---------|
| Components | HeroUI | `@heroui/react` |
| Styling | Tailwind CSS v4 | `tailwindcss` + `@tailwindcss/vite` |
| Theme | HeroUI plugin | `heroui()` in `hero.ts` |
| Animation | Framer Motion | `framer-motion` (>= 11.9) |
| Icons | Lucide | `lucide-react` |
| Font | Geist Sans + Geist Mono | `@imdanibytes/nexus-ui/styles` |
| Shared provider | NexusProvider | `@imdanibytes/nexus-ui` |

Plugins MUST wrap their root in `<NexusProvider>` from `@imdanibytes/nexus-ui`. This sets up HeroUI, dark mode, fonts, and the theme bridge.

---

## Core Concept: Layers of Glass

The entire UI is built on a single metaphor: **translucent paper layers floating over an animated gradient background.** Every visible surface is a frosted-glass panel. There are no opaque containers.

```
┌─────────────────────────────────────────────┐
│  Gradient Background (fixed, -z-10)         │
│  ┌──────┐ ┌──────────────────────────────┐  │
│  │Sidebar│ │ Main Content                 │  │
│  │ glass │ │  glass                       │  │
│  │  ┌──┐ │ │  ┌─────────────────────┐    │  │
│  │  │S1│ │ │  │ Surface panel (menu) │    │  │
│  │  └──┘ │ │  └─────────────────────┘    │  │
│  │  ┌──┐ │ │                             │  │
│  │  │S2│ │ │  Plugin iframe content      │  │
│  │  └──┘ │ │                             │  │
│  │  ┌──┐ │ │                             │  │
│  │  │S3│ │ │                             │  │
│  │  └──┘ │ │                             │  │
│  └──────┘ └──────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

### Background

Five large radial-gradient blobs (teal, violet, pink, blue, amber) drift slowly behind everything. Each blob is 700-900px, `blur-[160px]`, animating on 25-35s loops. Opacity adapts: `opacity-15 dark:opacity-60` (range).

Plugins don't render this — the shell does. But plugins should assume their background is **not solid black** — it's a dark translucent surface with color bleeding through.

### Glass Panels

Every container uses the same frosted-glass treatment:

```
rounded-xl bg-default-50/40 backdrop-blur-xl border border-white/5
```

That's it. One recipe. Used for:
- Sidebar section groups
- Plugin menu bar
- Settings card groups
- Any elevated container

> **Plugin authors:** Use `border-default-200/50` instead of `border-white/5` if your plugin supports both light and dark mode. The `white/5` border is invisible on light backgrounds. The shell app uses `white/5` because it's dark-mode only, but plugins receive theme changes and should handle both.

**Do not** use opaque backgrounds (`bg-default-50`, `bg-default-100`) for containers. Always use the translucent `bg-default-50/40` with backdrop blur.

### Shell Panels

The sidebar and main content area are larger glass panels:

```css
/* Sidebar */
backdrop-blur-2xl bg-background/40 p-3 gap-2

/* Main content */
backdrop-blur-2xl bg-background/40 border-l border-white/5
```

---

## Color System

We use **HeroUI semantic tokens** exclusively. No custom hex values, no Tailwind color palette (`slate-800`, `gray-700`, etc).

### Semantic Colors

| Token | Usage |
|-------|-------|
| `primary` | Brand accent. Active states, links, CTAs. HeroUI default teal. |
| `success` | Running, healthy, approved. Green. |
| `warning` | Installing, caution, pending. Amber. |
| `danger` | Error, failed, denied. Red. |
| `default` | Neutral. Backgrounds, borders, muted elements. |
| `foreground` | Primary text color. |
| `background` | Base background (used with opacity for glass). |

### Text Hierarchy

| Class | Usage |
|-------|-------|
| `text-foreground` | Primary text. Headings, names, active labels. |
| `text-foreground font-medium` | Emphasized primary text. Active nav items. |
| `text-default-500` | Secondary text. Descriptions, inactive labels, body text. |
| `text-default-400` | Muted text. Metadata, timestamps, helper text, icons at rest. |
| `text-primary` | Accent text. Links, brand dot, status highlights. |

### Backgrounds

| Class | Usage |
|-------|-------|
| `bg-default-50/40` | Glass panel fill (with `backdrop-blur-xl border border-white/5`) |
| `bg-default-100` | Active/selected state (expanded nav items) |
| `bg-default-50` | Hover state on interactive elements |
| `bg-primary/15` | Active/selected state (collapsed sidebar items) |
| `bg-background/40` | Shell-level glass (sidebar, main content) |

### Borders

**Shell app:** `border-white/5` — the shell is dark-mode only, so white at 5% opacity gives a subtle glass edge.

**Plugins:** `border-default-200/50` — plugins must support light and dark mode via theme sync. This semantic border adapts to both themes. Never use `border-divider` or raw `border-default`.

---

## Typography

### Fonts

**Sans:** Geist → Inter → system-ui (fallback chain)
**Mono:** Geist Mono → JetBrains Mono → SF Mono → ui-monospace

Plugins get these fonts automatically via `@imdanibytes/nexus-ui/styles`. If building standalone, install the `geist` npm package.

```css
font-family: "Geist", "Inter", system-ui, -apple-system, BlinkMacSystemFont, sans-serif;
```

### Scale

| Size | Class | Usage |
|------|-------|-------|
| 18px | `text-lg` | Page titles |
| 16px | `text-base` | Section headings, modal titles |
| 14px | `text-sm` | Nav items, plugin names, body text (default) |
| 13px | `text-[13px]` | Menu bar items, compact body text, descriptions |
| 12px | `text-xs` | Metadata, version numbers, badge text |
| 11px | `text-[11px]` | Captions, overlines, parameter labels |
| 10px | `text-[10px]` | Tiny indicators (e.g. "ON" badge) |

### Weights

| Weight | Class | Usage |
|--------|-------|-------|
| 700 | `font-bold` | Brand wordmark only |
| 600 | `font-semibold` | Page headings, section headers, active plugin name in menu bar |
| 500 | `font-medium` | Active nav items, emphasized labels |
| 400 | (default) | Everything else |

---

## Icons

**Library:** Lucide React (`lucide-react`)

| Context | Size | strokeWidth |
|---------|------|-------------|
| Nav items, inline buttons | 16px | 1.5 (default) |
| Menu bar start content | 14px | 1.5 |
| Status/action overlays | 28px | 1.5 |
| Small indicators | 12-13px | 1.5-2 |

Icons use `currentColor`. Set color via text utility classes (`text-default-400`, `text-primary`, etc).

---

## Interactive Elements

### Nav Items

A nav item is a full-width button with an icon (16px), a gap, and a text label.

```
┌──────────────────────────┐
│  [icon 16px] [gap] Label │   ← expanded
└──────────────────────────┘

┌──────┐
│[icon]│   ← collapsed (icon centered, text hidden)
└──────┘
```

**Classes:**

```
/* Base */
relative flex items-center w-full rounded-xl text-sm
transition-all duration-300

/* Expanded */
px-3 py-2 gap-3

/* Collapsed */
px-0 py-2 gap-0 justify-center

/* Active */
text-foreground font-medium
+ absolute inset-0 div with:
  expanded: bg-default-100 rounded-xl
  collapsed: bg-primary/15 rounded-xl

/* Inactive */
text-default-500 hover:text-foreground hover:bg-default-50
```

### Icon Slot Alignment

Any item with a leading visual element (icon, status dot, avatar letter) MUST use a **16px-wide slot** (`w-4 shrink-0`) so text labels align vertically across all items:

```
[  •  ] Cookie Jar      ← dot centered in w-4 container
[icon ] Add Plugins     ← 16px icon fills w-4 naturally
[  A  ] Agent Chat      ← letter centered in w-4 container
```

The gap between the icon slot and text is `gap-3` (12px) via the NavItem flex container.

### Status Dots

Status dots indicate plugin/extension state. They are 8px circles inside a 16px alignment container.

```html
<span class="flex items-center justify-center w-4 shrink-0">
  <span class="relative flex h-2 w-2">
    <!-- Ping layer (running only) -->
    <span class="absolute inline-flex h-full w-full rounded-full bg-success animate-ping opacity-75" />
    <!-- Solid dot -->
    <span class="relative inline-flex rounded-full h-2 w-2 bg-success" />
  </span>
</span>
```

| Status | Color | Animation |
|--------|-------|-----------|
| Running | `bg-success` | `animate-ping` until viewport is warm |
| Stopped | `bg-default-400` | None |
| Error | `bg-danger` | None |
| Installing | `bg-warning` | None |

### Buttons

Use HeroUI `<Button>` exclusively. Common patterns:

| Variant | Usage |
|---------|-------|
| `color="primary"` | Primary CTA ("Start Plugin", "Install") |
| `variant="flat"` | Secondary actions ("Cancel") |
| `color="danger"` | Destructive actions ("Remove") |
| `variant="flat" color="danger"` | Soft destructive ("Remove" in confirmation) |

### Menu Bars

Plugin viewport menu bars use the Surface glass panel treatment, floating inside the content area:

```
mx-2 mt-2 px-1 h-8 rounded-xl bg-default-50/40 backdrop-blur-xl border border-white/5
```

Menu items are plain `<button>` elements:

```
px-2 py-1 text-[13px] rounded hover:bg-default-200/40 transition-colors
```

The first item (plugin name) is `font-semibold`. Subsequent items are `text-default-500`.

### Dropdowns

Use HeroUI `<Dropdown>` + `<DropdownMenu>` + `<DropdownItem>`. Sections separated with `<DropdownSection showDivider>`.

Start content icons: 14px, colored by action type (success for start, warning for stop, danger for remove, primary for update/rebuild).

---

## Layout

### Sidebar

Collapsible. 240px expanded, 68px collapsed. Animated with Framer Motion spring (`bounce: 0, duration: 0.3`).

Three Surface sections stacked vertically:
1. **Brand** — "nexus." wordmark
2. **Plugins/Extensions** — scrollable list
3. **Navigation** — Add Plugins, Extensions, Settings

Collapse/expand uses **CSS transitions on the same DOM** — not conditional rendering. Text labels transition `w-0 opacity-0` ↔ `w-auto opacity-100` via `transition-all duration-300`. Icons stay in place. This prevents layout animation bugs.

### Content Area

Plugin content renders in stacked iframes. Only the selected plugin is visible; others use `invisible pointer-events-none`. Settings and marketplace use `opacity-0 pointer-events-none` to preserve state.

---

## Animation

### Motion Principles

| What | How | Duration |
|------|-----|----------|
| Sidebar width | Framer Motion spring | `bounce: 0, duration: 0.3` |
| Nav item state changes | CSS `transition-all` | 300ms |
| Text collapse/expand | CSS `transition-all` on width + opacity | 300ms |
| Button hover | CSS `transition-colors` | 150ms (Tailwind default) |
| Modal open/close | HeroUI built-in | Default |

**Do not** use Framer Motion `layoutId` on sidebar nav items — it causes animation bugs during collapse transitions. Use plain `div` elements for active indicators.

**Do not** conditionally render DOM during animated layout changes. Always keep elements mounted and use CSS transitions to show/hide.

---

## Plugin Iframe Guidelines

Plugins render inside sandboxed iframes in the Nexus viewport. To match the host app:

### Required Setup

1. Install: `@heroui/react`, `framer-motion`, `@imdanibytes/nexus-ui`, `lucide-react`
2. Wrap root in `<NexusProvider>` from `@imdanibytes/nexus-ui`
3. Import styles: `@import "@imdanibytes/nexus-ui/styles"` in your CSS
4. Set `body { background: transparent; }` so the host gradient bleeds through your glass panels
5. Use HeroUI's Tailwind plugin in your `hero.ts`:

```ts
import { heroui } from "@heroui/react";
export default heroui({ defaultTheme: "dark" });
```

### Theme Sync

Nexus sends theme changes via `postMessage`:

```ts
window.addEventListener("message", (e) => {
  if (e.data?.type === "nexus:system" && e.data.event === "theme_changed") {
    // e.data.data.theme is "light" | "dark"
    document.documentElement.className = e.data.data.theme;
  }
});
```

The initial theme is passed as a `?nexus_theme=dark` query parameter on the iframe URL.

### Visual Checklist for Plugins

- [ ] Dark mode is the default and looks correct
- [ ] Light mode also works (theme sync via `postMessage`)
- [ ] Body background is `transparent` (not opaque — the host gradient must bleed through)
- [ ] No opaque gray containers — use `bg-default-50/40 backdrop-blur-xl border border-default-200/50` for panels
- [ ] Text uses HeroUI semantic colors (`text-foreground`, `text-default-500`), not hardcoded hex
- [ ] Buttons are HeroUI `<Button>`, not custom styled
- [ ] Icons are Lucide at 16px / strokeWidth 1.5
- [ ] Font is Geist (inherited from NexusProvider styles)
- [ ] Scrollbars are thin (6px) with transparent track
- [ ] No competing accent colors — `primary` is the only accent
- [ ] Modals use HeroUI `<Modal>`, not custom overlays
- [ ] Forms use HeroUI inputs (built-in labels, descriptions, error states)

---

## Scrollbar

```css
::-webkit-scrollbar { width: 6px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: hsl(var(--heroui-default-300)); border-radius: 3px; }
::-webkit-scrollbar-thumb:hover { background: hsl(var(--heroui-default-400)); }
```

---

## Border Radius

| Element | Radius |
|---------|--------|
| Glass panels (Surface) | `rounded-xl` (12px) |
| Buttons, inputs | HeroUI defaults |
| Modals | `rounded-xl` (12px, HeroUI default) |
| Status dots | `rounded-full` |
| Nav item hover/active indicator | `rounded-xl` |
| Menu bar item hover | `rounded` (4px) |
| App icon placeholders | `rounded-[14px]` |

---

## Spacing

Base unit: 4px. Sidebar uses `p-3` (12px) internal padding. Glass panels use `p-2` (8px). Nav items use `px-3 py-2` (12px / 8px). Menu bar items use `px-2 py-1` (8px / 4px).

Section gaps: `gap-2` (8px) between Surface panels in sidebar. `space-y-0.5` (2px) between nav items within a panel.

---

## What NOT to Do

- **No opaque containers.** Every panel is translucent glass.
- **No Tailwind color palette** (`slate-*`, `gray-*`, `zinc-*`). Use HeroUI semantic tokens.
- **No custom hex colors** in components. Define them in the theme if needed.
- **No `border-divider`** for glass panel borders. Use `border-white/5` (shell) or `border-default-200/50` (plugins).
- **No heavy shadows** on cards. Depth comes from translucency and backdrop blur.
- **No conditional DOM rendering** during layout animations. CSS transitions only.
- **No Framer Motion `layoutId`** on elements that change during sidebar collapse.
- **No `justify-center`** on text labels. Text is always left-aligned in its flex container.
- **No `flex-1`** on text label spans — it changes text positioning. Use `truncate whitespace-nowrap`.
