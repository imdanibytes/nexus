# Nexus UX Design Direction

## Current State Analysis

Nexus currently uses a **Tailwind slate dark theme** with indigo-500 accents. Here is what is in play:

| Element | Current Value | Assessment |
|---------|---------------|------------|
| Body background | `#0f172a` (slate-900) | Fine but generic |
| Sidebar background | slate-800 | Flat, no depth hierarchy |
| Card backgrounds | slate-800 + slate-700 borders | Standard Tailwind starter kit |
| Accent color | indigo-500 | The default "I didn't pick a color" color |
| Text primary | `#e2e8f0` (slate-200) | Good readability |
| Text secondary | slate-400 | Decent hierarchy |
| Text muted | slate-500 | Gets lost on dark backgrounds |
| Status: running | green-500 | Fine |
| Status: error | red-500 | Fine |
| Status: installing | yellow-500 | Fine |
| Border radius | `rounded-lg` / `rounded-xl` | Inconsistent between components |
| Font | Inter + system-ui fallback | Solid but could be more intentional |
| Scrollbar | slate-700 thumb | Functional but invisible |

**The core problem:** This looks like every Tailwind dark mode tutorial. There is no visual identity. If you screenshot this and put it next to 20 other developer tool dashboards, you could not pick it out.

---

## 1. Color Palette

### Design Philosophy

Nexus is a **power user's control center** --- it manages Docker containers, plugins, permissions, system resources. The palette should feel:

- **Authoritative** --- this is a command bridge, not a toy
- **Warm-dark** --- not cold blue-gray, not sterile, not oppressive
- **High-signal** --- accent colors mean something, they are not decorative

The strategy: shift from Tailwind's **slate** (blue-gray) to a **custom neutral with warm undertones**, and replace indigo with a distinctive **teal-cyan accent** that feels technical and alive without being cold.

### Surface / Background Scale

Moving away from slate's blue cast toward a warmer, more neutral dark palette. These are based on a near-neutral base with a subtle warm-green undertone (think matte black dashboard, not blue terminal).

| Token | Hex | Usage |
|-------|-----|-------|
| `bg-deep` | `#0C0E12` | App background, deepest layer |
| `bg-base` | `#12141A` | Main content area background |
| `bg-surface` | `#1A1D25` | Cards, panels, elevated containers |
| `bg-raised` | `#222631` | Sidebar, headers, secondary surfaces |
| `bg-overlay` | `#2A2E3A` | Dropdowns, popovers, hover states on cards |
| `bg-wash` | `#323744` | Input fields, well backgrounds, inset areas |

These have ~3-4% brightness steps between them, enough for the eye to distinguish layers without looking stripey.

### Accent Colors

The primary accent is a **teal-cyan** --- technical, energetic, distinct from the indigo/violet that every developer tool and its dog uses now. A secondary **amber** provides warmth and draws attention for interactive elements.

| Token | Hex | Usage |
|-------|-----|-------|
| `accent` | `#2DD4A8` | Primary accent: active nav, selected states, primary buttons |
| `accent-hover` | `#22B893` | Hover state for primary accent |
| `accent-muted` | `#2DD4A8` at 15% opacity | Active nav backgrounds, badge backgrounds |
| `accent-subtle` | `#2DD4A8` at 8% opacity | Large selected area backgrounds |
| `highlight` | `#F0AB3A` | Secondary accent: attention-drawing, badges, new indicators |
| `highlight-muted` | `#F0AB3A` at 15% opacity | Background for highlights |

### Text Colors

| Token | Hex | Usage |
|-------|-----|-------|
| `text-primary` | `#E8ECF2` | Headings, primary content, names |
| `text-secondary` | `#9BA3B2` | Body text, descriptions |
| `text-muted` | `#5E6778` | Metadata, timestamps, helper text |
| `text-ghost` | `#3D4452` | Disabled text, placeholders |
| `text-on-accent` | `#0C0E12` | Text on accent-colored backgrounds |
| `text-link` | `#2DD4A8` | Interactive text links |

### Status Colors

These stay close to universal conventions but are tuned for the dark background and slightly desaturated so they do not scream.

| Token | Hex | Tinted BG | Usage |
|-------|-----|-----------|-------|
| `status-success` | `#34D399` | `rgba(52,211,153,0.12)` | Running, healthy, approved |
| `status-warning` | `#FBBF24` | `rgba(251,191,36,0.12)` | Installing, caution, pending |
| `status-error` | `#F87171` | `rgba(248,113,113,0.12)` | Error, failed, denied |
| `status-info` | `#60A5FA` | `rgba(96,165,250,0.12)` | Informational, neutral status |

### Border Colors

| Token | Hex | Usage |
|-------|-----|-------|
| `border-default` | `#2A2E3A` | Card borders, dividers |
| `border-subtle` | `#222631` | Very faint separation lines |
| `border-strong` | `#3D4452` | Input borders, focused dividers |
| `border-accent` | `#2DD4A8` at 50% opacity | Selected card borders, focus rings |

---

## 2. Typography

### Font Stack

**UI / Sans-serif: Geist Sans**

Geist (by Vercel/Basement Studio) is the right call here. It was designed for exactly this kind of interface --- developer tools, dashboards, dense information display. It has tighter letter-spacing than Inter at small sizes, which matters when you are showing plugin names, version numbers, Docker image tags, and permission labels in constrained sidebar space.

- Excellent legibility at 11-14px (the range this app lives in)
- Designed for screen rendering, not print
- Distinctive enough to not look like "generic sans-serif" but neutral enough to not distract
- Available on Google Fonts and as an npm package (`geist`)

If Geist is not available or creates bundling issues, fall back to Inter. It is still good. But Geist is better for this use case.

**Monospace: Geist Mono**

For code elements (Docker image names, version strings, log output, JSON manifests), Geist Mono pairs naturally with Geist Sans. If not available, JetBrains Mono is the fallback.

```css
:root {
  --font-sans: 'Geist', 'Inter', system-ui, -apple-system, sans-serif;
  --font-mono: 'Geist Mono', 'JetBrains Mono', 'SF Mono', 'Fira Code', ui-monospace, monospace;
}
```

### Type Scale

The current app uses `text-xs`, `text-sm`, `text-lg` somewhat arbitrarily. Here is a deliberate scale:

| Token | Size | Weight | Line Height | Usage |
|-------|------|--------|-------------|-------|
| `heading-lg` | 18px | 700 | 1.3 | Page titles ("Settings", "Add Plugins") |
| `heading-sm` | 14px | 600 | 1.4 | Section headers ("Docker", "About") |
| `body` | 13px | 400 | 1.5 | Primary body text, descriptions |
| `label` | 12px | 500 | 1.4 | Input labels, nav items, plugin names |
| `caption` | 11px | 400 | 1.4 | Metadata, timestamps, version numbers |
| `overline` | 10px | 600 | 1.2 | Section overlines ("INSTALLED"), tracking-wider, uppercase |
| `code` | 12px | 400 | 1.5 | Code/mono elements (image names, paths, logs) |

Key change: the current `text-lg font-bold` headings (18px) are fine, but the body text hovers between `text-xs` (12px) and `text-sm` (14px) without consistency. Pin body to 13px, metadata to 11px, labels to 12px.

---

## 3. Design Language

### Border Radius

**Philosophy: Softened, not bubbly.**

The current codebase mixes `rounded-lg` (8px) and `rounded-xl` (12px). Pick a system and commit:

| Element | Radius | Rationale |
|---------|--------|-----------|
| Cards, panels, sections | 10px (`rounded-[10px]` or define custom `rounded-card`) | Main container shape |
| Buttons, inputs, badges | 8px (`rounded-lg`) | Interactive elements, slightly tighter |
| Status dots | 9999px (`rounded-full`) | Circles stay circles |
| Modals / dialogs | 14px | Larger elements get slightly softer corners |
| Toast notifications | 10px | Matches cards |
| Category tags, pills | 6px (`rounded-md`) | Small inline elements |

Do NOT use `rounded-2xl` (16px) or `rounded-3xl` anywhere. This is a power tool, not a social media app.

### Spacing Rhythm

Base unit: **4px**. Every spacing value should be a multiple of 4.

| Usage | Value | Tailwind |
|-------|-------|----------|
| Tight inline gap | 4px | `gap-1` |
| Related element gap | 8px | `gap-2` |
| Component internal padding | 12-16px | `p-3` / `p-4` |
| Section padding | 20-24px | `p-5` / `p-6` |
| Page margins | 24px | `p-6` |
| Between sections | 24px | `space-y-6` |
| Sidebar internal | 12px horizontal, 12px vertical | `px-3 py-3` |

The current spacing is already mostly on this grid. The main fix is consistency --- some cards use `p-4`, some `p-5`, some `p-6`.

### Shadows and Depth

**Philosophy: Minimal shadows, use brightness for hierarchy.**

In a dark UI, traditional box-shadows are nearly invisible. Instead, depth comes from background brightness differences (the surface scale above) and subtle borders.

| Element | Shadow | Notes |
|---------|--------|-------|
| Cards at rest | None | Use `bg-surface` + `border-default` for separation |
| Cards on hover | `0 0 0 1px rgba(45,212,168,0.15)` | Faint accent glow border, not a shadow |
| Dropdowns / popovers | `0 8px 24px rgba(0,0,0,0.4)` | Real shadow for floating elements |
| Modals | `0 16px 48px rgba(0,0,0,0.5)` | Heavier shadow + backdrop blur |
| Toasts | `0 4px 16px rgba(0,0,0,0.3)` | Moderate lift |
| Focused inputs | `0 0 0 2px rgba(45,212,168,0.3)` | Accent ring, not shadow |

### Glass / Blur Effects

**Use sparingly. One or two surfaces, max.**

Good candidates:
- **Modal backdrop**: `backdrop-blur-sm` (4px) on the dark overlay --- makes the modal feel like it floats above the content
- **Sidebar**: A very subtle `backdrop-blur-md` (12px) with `bg-raised/80` opacity can add dimension if the main content behind it is visually active (like an embedded plugin UI in an iframe)

Bad candidates:
- Cards (too many blur layers kills performance, especially in a Tauri webview)
- Toast notifications (they are temporary and do not need frosted glass)
- Every elevated surface (that is just a blurry mess)

```css
/* Sidebar with subtle glass effect */
.sidebar-glass {
  background: rgba(34, 38, 49, 0.85);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}

/* Modal backdrop */
.modal-backdrop {
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(4px);
}
```

### Micro-interactions

| Interaction | Duration | Easing | Properties |
|-------------|----------|--------|------------|
| Button hover | 150ms | ease-out | background-color, border-color |
| Card hover | 200ms | ease-out | border-color, box-shadow |
| Nav item active | 150ms | ease-out | background-color, color |
| Modal open | 200ms | ease-out | opacity, transform (scale 0.98 -> 1) |
| Modal close | 150ms | ease-in | opacity, transform |
| Toast enter | 300ms | cubic-bezier(0.16,1,0.3,1) | transform (translateY 100% -> 0), opacity |
| Toast exit | 200ms | ease-in | opacity, transform |
| Status dot pulse | 2s | ease-in-out, infinite | opacity (0.5 -> 1 -> 0.5) |
| Focus ring | 150ms | ease-out | box-shadow |
| Sidebar collapse | 200ms | ease-out | width |

Current state: the app uses `transition-colors` everywhere, which is fine for the basics. The missing pieces are:
- No entrance/exit animations on modals or toasts
- No hover feedback on cards beyond border color change
- Status dots are static (a subtle pulse on `running` would add life)

### Icon Style

**Recommendation: Lucide (outline, 1.5px stroke)**

The current codebase uses inline SVG with `stroke="currentColor"` and `strokeWidth={2}`. This is the right approach (outline icons), but the stroke weight of 2 feels heavy at 16x16px.

Switch to [Lucide](https://lucide.dev/) icons:
- Same icon language as Heroicons (which the current SVGs approximate) but more complete
- Default 1.5px stroke looks cleaner at small sizes
- React package: `lucide-react` --- tree-shakeable, no bundle bloat
- Consistent 24x24 viewBox with optical adjustments for each icon

For status indicators and small badges, consider **dual-tone**: outline icon with a filled accent element. Example: a container icon (outline) with a green-filled circle overlay for "running."

---

## 4. Inspiration References

### Primary Inspirations

**1. Warp Terminal** (warp.dev)
- Why: Dark developer tool with an opinionated, non-generic aesthetic. Their teal/cyan accent on dark neutral backgrounds is close to what Nexus should aim for. The way they handle "UI surfaces" (consistent elevated backgrounds for overlays) is well-executed.
- Take from it: Accent color confidence, surface layering philosophy, the warm-neutral-not-cold-blue background approach.

**2. Linear** (linear.app)
- Why: The gold standard for dark-mode developer/productivity tools in 2024-2025. Their LCH-based color system generates beautiful gradients from just 3 base values. Their information density is high but never cluttered.
- Take from it: Typography hierarchy, information density management, the way active/selected states use a muted accent background rather than a heavy fill.

**3. Raycast** (raycast.com)
- Why: macOS-native feel. Since Nexus is a Tauri app on macOS, it should feel like it belongs on macOS. Raycast's command palette UI, subtle blur effects, and tight spacing are a masterclass in desktop-native design for developer tools.
- Take from it: macOS-native spacing rhythm, subtle glass/blur usage that enhances without overwhelming, icon density and sizing.

**4. Vercel Dashboard** (vercel.com/dashboard)
- Why: Uses Geist (the font recommended above), has a methodical color token system, and handles complex status information (deployments, builds, domains) with clarity. Their dark mode is one of the best implementations of neutral-warm backgrounds.
- Take from it: The Geist type scale in action, status color treatment, the specific gray scale (not too blue, not too warm).

**5. Docker Desktop**
- Why: Direct competitor/adjacent product. Nexus manages Docker containers, so users will subconsciously compare the two. Docker Desktop's dark mode is decent but bland --- Nexus should be clearly better and more opinionated.
- Take from it: What NOT to do --- their gray-on-gray hierarchy is too flat, their accent (Docker blue) gets lost. Learn from their container status visualization patterns, but execute them with more visual confidence.

### Secondary References

- **Obsidian** --- For plugin ecosystem UI patterns (marketplace, settings, permissions)
- **Arc Browser** --- For sidebar navigation patterns and how to make dense sidebars feel spacious
- **GitHub Desktop** --- For macOS-native Electron/Tauri-style desktop app conventions

---

## 5. Tailwind v4 Theme Configuration

Tailwind v4 uses CSS-based configuration rather than `tailwind.config.ts`. Here is the proposed theme, defined as CSS custom properties in `index.css`:

```css
@import "tailwindcss";

/* ============================================
   Nexus Design Tokens
   ============================================ */

@theme {
  /* --- Surface / Background --- */
  --color-nx-deep: #0C0E12;
  --color-nx-base: #12141A;
  --color-nx-surface: #1A1D25;
  --color-nx-raised: #222631;
  --color-nx-overlay: #2A2E3A;
  --color-nx-wash: #323744;

  /* --- Accent --- */
  --color-nx-accent: #2DD4A8;
  --color-nx-accent-hover: #22B893;
  --color-nx-accent-muted: oklch(0.72 0.15 168 / 0.15);
  --color-nx-accent-subtle: oklch(0.72 0.15 168 / 0.08);

  /* --- Highlight (secondary accent) --- */
  --color-nx-highlight: #F0AB3A;
  --color-nx-highlight-muted: oklch(0.78 0.15 75 / 0.15);

  /* --- Text --- */
  --color-nx-text: #E8ECF2;
  --color-nx-text-secondary: #9BA3B2;
  --color-nx-text-muted: #5E6778;
  --color-nx-text-ghost: #3D4452;

  /* --- Border --- */
  --color-nx-border: #2A2E3A;
  --color-nx-border-subtle: #222631;
  --color-nx-border-strong: #3D4452;
  --color-nx-border-accent: oklch(0.72 0.15 168 / 0.5);

  /* --- Status --- */
  --color-nx-success: #34D399;
  --color-nx-success-muted: oklch(0.73 0.17 163 / 0.12);
  --color-nx-warning: #FBBF24;
  --color-nx-warning-muted: oklch(0.83 0.16 85 / 0.12);
  --color-nx-error: #F87171;
  --color-nx-error-muted: oklch(0.68 0.19 22 / 0.12);
  --color-nx-info: #60A5FA;
  --color-nx-info-muted: oklch(0.7 0.14 250 / 0.12);

  /* --- Shadows --- */
  --shadow-dropdown: 0 8px 24px rgba(0, 0, 0, 0.4);
  --shadow-modal: 0 16px 48px rgba(0, 0, 0, 0.5);
  --shadow-toast: 0 4px 16px rgba(0, 0, 0, 0.3);
  --shadow-focus: 0 0 0 2px oklch(0.72 0.15 168 / 0.3);
  --shadow-card-hover: 0 0 0 1px oklch(0.72 0.15 168 / 0.15);

  /* --- Border Radius --- */
  --radius-card: 10px;
  --radius-button: 8px;
  --radius-input: 8px;
  --radius-modal: 14px;
  --radius-tag: 6px;

  /* --- Typography --- */
  --font-sans: 'Geist', 'Inter', system-ui, -apple-system, BlinkMacSystemFont, sans-serif;
  --font-mono: 'Geist Mono', 'JetBrains Mono', 'SF Mono', ui-monospace, monospace;

  /* --- Transitions --- */
  --transition-fast: 150ms ease-out;
  --transition-normal: 200ms ease-out;
  --transition-slow: 300ms cubic-bezier(0.16, 1, 0.3, 1);
}

/* ============================================
   Base Styles
   ============================================ */

:root {
  font-family: var(--font-sans);
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

body {
  margin: 0;
  min-height: 100vh;
  background-color: var(--color-nx-deep);
  color: var(--color-nx-text);
}

#root {
  height: 100vh;
  overflow: hidden;
}

/* ============================================
   Scrollbar
   ============================================ */

::-webkit-scrollbar {
  width: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: var(--color-nx-wash);
  border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
  background: var(--color-nx-text-muted);
}
```

### Usage Examples (Tailwind Classes)

With the above tokens defined, here is how they map to component usage:

```
Shell background:          bg-nx-deep
Main content area:         bg-nx-base
Sidebar:                   bg-nx-raised border-r border-nx-border
Cards:                     bg-nx-surface border border-nx-border rounded-[var(--radius-card)]
Card hover:                hover:border-nx-border-strong hover:shadow-[var(--shadow-card-hover)]
Selected card:             border-nx-border-accent bg-nx-accent-subtle
Primary button:            bg-nx-accent hover:bg-nx-accent-hover text-nx-deep rounded-[var(--radius-button)]
Secondary button:          bg-nx-overlay hover:bg-nx-wash text-nx-text-secondary rounded-[var(--radius-button)]
Input:                     bg-nx-wash border border-nx-border-strong focus:shadow-[var(--shadow-focus)]
Active nav item bg:        bg-nx-accent-muted text-nx-accent
Heading text:              text-nx-text font-semibold
Body text:                 text-nx-text-secondary
Muted text:                text-nx-text-muted
Status running badge:      bg-nx-success-muted text-nx-success
Status error badge:        bg-nx-error-muted text-nx-error
Status warning badge:      bg-nx-warning-muted text-nx-warning
Modal overlay:             bg-black/50 backdrop-blur-sm
Modal container:           bg-nx-surface border border-nx-border rounded-[var(--radius-modal)] shadow-[var(--shadow-modal)]
Toast:                     bg-nx-raised border border-nx-border rounded-[var(--radius-card)] shadow-[var(--shadow-toast)]
Code elements:             font-mono bg-nx-deep text-nx-text-secondary px-1.5 py-0.5 rounded-[var(--radius-tag)]
```

---

## 6. Component-Specific Recommendations

### Sidebar (`Sidebar.tsx`)

- Background: `bg-nx-raised` (not the same as the content area --- creates depth)
- Border: `border-r border-nx-border`
- Logo area: consider a subtle gradient or the accent color in the "Nexus" text
- Selected plugin item: `bg-nx-accent-muted text-nx-accent` (teal tint, not white-on-gray)
- Status dots: keep them small (6px) but add a subtle CSS animation for `running`:

```css
@keyframes pulse-status {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
.status-running {
  animation: pulse-status 2s ease-in-out infinite;
}
```

### Plugin Cards (`PluginCard.tsx`)

- At rest: `bg-nx-surface border-nx-border`
- Hover: `border-nx-border-strong` + very faint accent glow
- Selected: `border-nx-border-accent bg-nx-accent-subtle`
- Status badges should use the `*-muted` background variants with the solid status color for text
- Category tags: `bg-nx-overlay text-nx-text-secondary rounded-[var(--radius-tag)]`

### Plugin Viewport (`PluginViewport.tsx`)

- Header bar: `bg-nx-raised/60 border-b border-nx-border` (slightly transparent so the iframe content bleeds through faintly --- creates depth)
- The "Start Plugin" CTA button: `bg-nx-accent hover:bg-nx-accent-hover text-nx-deep` (dark text on teal, high contrast)

### Logs Panel (`PluginLogs.tsx`)

- Container: `bg-nx-deep` (the darkest surface --- logs should feel like a terminal)
- Log text: `font-mono text-[12px] text-nx-text-secondary`
- Consider syntax-highlighting log levels (ERROR in `text-nx-error`, WARN in `text-nx-warning`, INFO in `text-nx-info`)

### Permission Dialog (`PermissionDialog.tsx`)

- Modal: `bg-nx-surface border-nx-border rounded-[var(--radius-modal)] shadow-[var(--shadow-modal)]`
- Risk badge colors map directly: `low` -> success, `medium` -> warning, `high` -> error

### Toast Notifications (`Shell.tsx`)

- All toasts: same base appearance (`bg-nx-raised border border-nx-border`) with a **colored left border** (4px) indicating type
- Error: left border `border-l-4 border-l-nx-error`
- Success: left border `border-l-4 border-l-nx-success`
- Info: left border `border-l-4 border-l-nx-info`
- This is more subtle than the current approach (fully colored background) and scales better visually

---

## 7. Migration Strategy

Applying this design direction does not require a big-bang rewrite. Here is the phased approach:

### Phase 1: Foundation (index.css + Shell)
1. Add the `@theme` block to `index.css`
2. Install Geist font (`pnpm add geist`)
3. Update `Shell.tsx` and `Sidebar.tsx` to use the new tokens
4. Update body/root styles

### Phase 2: Components
1. Update `PluginCard.tsx` (both variants)
2. Update `PluginControls.tsx`
3. Update `PluginViewport.tsx`
4. Update `SearchBar.tsx`

### Phase 3: Overlays and Details
1. Update `PluginLogs.tsx`
2. Update `PermissionDialog.tsx`
3. Update `PluginDetail.tsx`
4. Update `SettingsPage.tsx`, `DockerSettings.tsx`, `UpdateCheck.tsx`

### Phase 4: Polish
1. Add micro-interactions (modal transitions, toast animations, status pulse)
2. Add Lucide icons (replace inline SVGs)
3. Add sidebar glass effect
4. Final spacing/typography audit

Each phase is independently shippable. The tokens are additive --- old slate-* classes still work during migration.

---

## Sources

- [Dark Mode Color Palettes: Complete Guide (MyPaletteTool)](https://mypalettetool.com/blog/dark-mode-color-palettes)
- [Modern App Colors: Design Palettes That Work In 2026 (WebOsmotic)](https://webosmotic.com/blog/modern-app-colors/)
- [Best Dashboard Design Examples & Inspirations for 2026 (Muzli)](https://muz.li/blog/best-dashboard-design-examples-inspirations-for-2026/)
- [How we redesigned the Linear UI (Linear)](https://linear.app/now/how-we-redesigned-the-linear-ui)
- [Warp: How we designed themes for the terminal (Warp)](https://www.warp.dev/blog/how-we-designed-themes-for-the-terminal-a-peek-into-our-process)
- [Geist Design System (Vercel)](https://vercel.com/geist/introduction)
- [Geist Colors (Vercel)](https://vercel.com/geist/colors)
- [Geist Font (Vercel)](https://vercel.com/font)
- [Custom Themes - Warp Documentation](https://docs.warp.dev/terminal/appearance/custom-themes)
- [28 Best Free Fonts for Modern UI Design (Untitled UI)](https://www.untitledui.com/blog/best-free-fonts)
- [Best Code Fonts for Developers & Programmers (JHK Blog)](https://www.jhkinfotech.com/blog/code-fonts-for-developers-programmers)
- [Theme Variables - Tailwind CSS v4](https://tailwindcss.com/docs/theme)
- [Dark Mode - Tailwind CSS](https://tailwindcss.com/docs/dark-mode)
- [Tailwind v4 Custom Theme Styling (Flagrant)](https://www.beflagrant.com/blog/tailwindcss-v4-custom-theme-styling-2025-08-21)
- [Colors in Every Format - shadcn/ui](https://ui.shadcn.com/colors)
- [Theming - shadcn/ui](https://ui.shadcn.com/docs/theming)
- [Dark Mode Color Palettes for Modern Websites (Colorhero)](https://colorhero.io/blog/dark-mode-color-palettes-2025)
