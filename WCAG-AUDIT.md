# WCAG 2.1 Level AA Accessibility Audit

Automated pattern-based audit of the Nexus app frontend (`src/` and `packages/nexus-ui/src/`).
Conducted 2026-02-20.

---

## 1. Icon-Only Buttons Without `aria-label`

Buttons that contain only an icon (no visible text) and lack `aria-label`, `aria-labelledby`, or an equivalent accessible name.

| # | File | Line(s) | WCAG Criterion | Description | Severity |
|---|------|---------|----------------|-------------|----------|
| 1.1 | `src/components/plugins/PluginLogs.tsx` | 82-87 | 4.1.2 Name, Role, Value | Close button (`<X>` icon) in the log drawer uses `isIconOnly` with no `aria-label`. Tooltip is absent. Screen readers will announce an empty button. | Critical |
| 1.2 | `src/components/layout/Sidebar.tsx` | 206-211 | 4.1.2 Name, Role, Value | Plugin context menu trigger (`<MoreHorizontal>` icon) is a raw `<button>` with no `aria-label`. The only accessible content is a sibling `<span className="sr-only">menu</span>` rendered inside a `DropdownTrigger`, not on this button itself. | Major |
| 1.3 | `src/components/layout/Sidebar.tsx` | 439-443 | 4.1.2 Name, Role, Value | Extension context menu trigger (`<MoreHorizontal>` icon) -- same issue as 1.2. | Major |
| 1.4 | `src/components/layout/Sidebar.tsx` | 711-720 | 4.1.2 Name, Role, Value | Sidebar collapse toggle (`<PanelLeftOpen>` / `<PanelLeftClose>` icon) is a raw `<button>` with no `aria-label` or visible text. | Major |
| 1.5 | `src/components/marketplace/McpWrapWizard.tsx` | 176-181 | 4.1.2 Name, Role, Value | Close button (`<X>` icon) in the MCP Wrap Wizard header uses `isIconOnly` with no `aria-label`. | Critical |
| 1.6 | `src/components/settings/McpTab.tsx` | 244-251 | 4.1.2 Name, Role, Value | API key show/hide toggle (`<Eye>`/`<EyeOff>` icon) uses `isIconOnly` with no `aria-label`. The surrounding `<Tooltip>` does not propagate to the accessible name. | Major |
| 1.7 | `src/components/settings/McpTab.tsx` | 254-261 | 4.1.2 Name, Role, Value | API key copy button (`<Copy>`/`<Check>` icon) uses `isIconOnly` with no `aria-label`. | Major |
| 1.8 | `src/components/settings/McpTab.tsx` | 264-272 | 4.1.2 Name, Role, Value | API key regenerate button (`<RefreshCw>` icon) uses `isIconOnly` with no `aria-label`. | Major |
| 1.9 | `src/components/permissions/PermissionList.tsx` | 321-334 | 4.1.2 Name, Role, Value | Remove-path button (`<X>` icon) uses `isIconOnly` with `title` attribute but no `aria-label`. The `title` attribute is not consistently announced by screen readers. | Minor |
| 1.10 | `src/components/marketplace/McpWrapWizard.tsx` | 405-417 | 4.1.2 Name, Role, Value | Tool include/exclude toggle button in ToolsStep (`<Eye>`/`<EyeOff>` icon) is a raw `<button>` with no `aria-label`. | Major |
| 1.11 | `packages/nexus-ui/src/components/SettingsShell.tsx` | 40-46 | 4.1.2 Name, Role, Value | Close button (`<XIcon>`) in modal variant settings shell has no `aria-label`. | Major |

---

## 2. Color-Only Status Indicators

Components that convey status purely through color with no text alternative, violating requirements for users who cannot distinguish colors.

| # | File | Line(s) | WCAG Criterion | Description | Severity |
|---|------|---------|----------------|-------------|----------|
| 2.1 | `packages/nexus-ui/src/components/StatusDot.tsx` | 20-43 | 1.4.1 Use of Color; 1.1.1 Non-text Content | `StatusDot` renders a colored dot (green/grey/red/yellow) with no text label, `aria-label`, or `title`. Status is conveyed entirely through color. Used in the sidebar for every plugin. | Critical |
| 2.2 | `src/components/layout/Sidebar.tsx` | 172-179 | 1.4.1 Use of Color | Collapsed plugin item renders a status-colored `<span>` dot with no text alternative. | Major |
| 2.3 | `src/components/layout/Sidebar.tsx` | 379-387 | 1.4.1 Use of Color | Extension status in `ExtensionItem` renders a plain `<span>` dot (green/grey) with no text or ARIA attribute. | Major |
| 2.4 | `src/components/layout/Sidebar.tsx` | 404-409 | 1.4.1 Use of Color | Collapsed extension item renders a status-colored badge dot with no text. | Major |
| 2.5 | `src/components/settings/SystemTab.tsx` | 170-184 | 1.4.1 Use of Color | Docker engine status indicator is a colored `<span>` dot (green/yellow/red) with no text alternative. The adjacent `<Chip>` provides a text status, but the dot itself conveys a separate visual meaning with no accessible mapping. | Minor |
| 2.6 | `src/components/settings/McpTab.tsx` | 200-211 | 1.4.1 Use of Color | MCP gateway active/disabled indicator uses a `<CircleDot>` icon with color classes (`text-success`/`text-default-400`) as the primary indicator. Text is adjacent, which partially mitigates the issue. | Minor |
| 2.7 | `src/components/settings/McpTab.tsx` | 345-351 | 1.4.1 Use of Color | Plugin running indicator in the tool registry uses `<CircleDot>` with color but no text label for running/stopped state. | Major |
| 2.8 | `src/components/settings/SettingsPage.tsx` | 62-67 | 1.4.1 Use of Color | `TabDot` notification badge is a colored dot (`bg-primary`) with no text or ARIA label. | Minor |
| 2.9 | `src/components/layout/Sidebar.tsx` | 696-698 | 1.4.1 Use of Color | Settings notification badge (collapsed mode) is a colored dot with no text. | Minor |
| 2.10 | `packages/nexus-ui/src/components/ToolFallback.tsx` | 87-95 | 1.4.1 Use of Color | `StatusDot` in ToolFallback (complete=green, incomplete=red) conveys status purely through color. | Major |

---

## 3. Missing Form Label Associations

Form inputs (`<input>`, `<select>`, `<textarea>`) without a properly associated `<label htmlFor>` or `aria-label`/`aria-labelledby`.

| # | File | Line(s) | WCAG Criterion | Description | Severity |
|---|------|---------|----------------|-------------|----------|
| 3.1 | `src/components/settings/SystemTab.tsx` | 315-322 | 1.3.1 Info and Relationships; 4.1.2 Name, Role, Value | CPU limit `<input type="range">` has an adjacent `<label>` on line 311 but without `htmlFor` or `id` binding. The label and input are not programmatically associated. | Major |
| 3.2 | `src/components/settings/SystemTab.tsx` | 337-345 | 1.3.1 Info and Relationships | Memory limit `<Input type="number">` (HeroUI) has an adjacent `<label>` (line 334) but without `htmlFor`/`id` binding. | Major |
| 3.3 | `src/components/settings/RegistrySettings.tsx` | 93-98 | 1.3.1 Info and Relationships | Registry name `<Input>` has an adjacent `<label>` (line 90) but without `htmlFor`/`id` association. HeroUI `Input` does not auto-associate with sibling labels. | Major |
| 3.4 | `src/components/settings/RegistrySettings.tsx` | 123-132 | 1.3.1 Info and Relationships | Registry URL `<Input>` has an adjacent `<label>` (line 120) but without `htmlFor`/`id` association. | Major |
| 3.5 | `src/components/marketplace/McpWrapWizard.tsx` | 290-306 | 1.3.1 Info and Relationships | MCP server command `<Input>` in CommandStep has an adjacent `<label>` (line 287) but no `htmlFor`/`id` binding. | Major |
| 3.6 | `src/components/marketplace/McpWrapWizard.tsx` | 698-713 | 1.3.1 Info and Relationships | All `FieldInput` instances (plugin ID, name, description, author) use `<label>` without `htmlFor`. The HeroUI `Input` inside has no `id` prop. | Major |
| 3.7 | `src/components/settings/SecurityTab.tsx` | 179-192 | 1.3.1 Info and Relationships | Plugin filter `<Input>` in the security tab has no label or `aria-label`. The `placeholder` is the only hint, which is insufficient per WCAG (placeholders disappear on input). | Major |
| 3.8 | `src/components/settings/McpTab.tsx` | 196 | 4.1.2 Name, Role, Value | Global MCP toggle `<Switch>` has no label or `aria-label`. It is visually near the heading but not programmatically associated. | Major |
| 3.9 | `src/components/settings/McpTab.tsx` | 373 | 4.1.2 Name, Role, Value | Plugin-level toggle `<Switch>` in the tool registry has no `aria-label`. | Major |
| 3.10 | `src/components/settings/McpTab.tsx` | 434 | 4.1.2 Name, Role, Value | Individual tool toggle `<Switch>` has no `aria-label`. | Major |
| 3.11 | `src/components/marketplace/McpWrapWizard.tsx` | 584-588 | 4.1.2 Name, Role, Value | Permission toggle `<Switch>` in McpWrapWizard PermissionsStep has no `aria-label` (unlike the PermissionDialog toggle which does). | Major |
| 3.12 | `src/components/settings/RegistrySettings.tsx` | 159 | 4.1.2 Name, Role, Value | Registry enable/disable `<Switch>` has no `aria-label` or associated label. | Major |

---

## 4. Expandable Sections Without ARIA

Collapsible/expandable UI patterns missing `aria-expanded`, `aria-controls`, or appropriate `role` attributes.

| # | File | Line(s) | WCAG Criterion | Description | Severity |
|---|------|---------|----------------|-------------|----------|
| 4.1 | `src/components/settings/McpTab.tsx` | 341-370 | 4.1.2 Name, Role, Value | Plugin group expand/collapse button lacks `aria-expanded` and `aria-controls`. The chevron rotation is the only visual cue; screen readers get no state information. | Major |
| 4.2 | `src/components/settings/ExtensionsTab.tsx` | 109-152 | 4.1.2 Name, Role, Value | Extension card expand/collapse button lacks `aria-expanded` and `aria-controls`. | Major |
| 4.3 | `src/components/settings/SecurityTab.tsx` | 208-240 | 4.1.2 Name, Role, Value | Plugin permission expand/collapse (`CardBody as="button"`) lacks `aria-expanded`. The `ChevronDown` rotation indicates state visually only. | Major |
| 4.4 | `src/components/permissions/PermissionList.tsx` | 249-252 | 4.1.2 Name, Role, Value | Active permission row expand/collapse for filesystem paths (click handler on a `<div>`) lacks `role="button"`, `aria-expanded`, and `aria-controls`. | Major |
| 4.5 | `src/components/settings/UpdateCheck.tsx` | 205-222 | 4.1.2 Name, Role, Value | Release notes expand/collapse button lacks `aria-expanded`. | Minor |
| 4.6 | `packages/nexus-ui/src/components/ToolFallback.tsx` | 46-67 | 4.1.2 Name, Role, Value | `ToolFallbackRoot` uses `onClick` on a `<div>` for expand/collapse behavior. It lacks `role="button"`, `tabIndex`, `aria-expanded`, and keyboard event handlers (`onKeyDown` for Enter/Space). | Critical |

---

## 5. Keyboard-Inaccessible Elements

Interactive elements that only respond to mouse events or use non-focusable elements for interactive behavior, without keyboard equivalents.

| # | File | Line(s) | WCAG Criterion | Description | Severity |
|---|------|---------|----------------|-------------|----------|
| 5.1 | `src/components/permissions/PermissionList.tsx` | 249-252 | 2.1.1 Keyboard | The active permission row is a `<div>` with `onClick` handler but no `tabIndex`, `role="button"`, or `onKeyDown`. Keyboard users cannot expand filesystem paths. | Critical |
| 5.2 | `src/components/marketplace/McpWrapWizard.tsx` | 378-434 | 2.1.1 Keyboard | Tool cards in ToolsStep are `<div>` elements with `onClick` handler but no `tabIndex`, `role`, or `onKeyDown`. Keyboard users cannot toggle tool inclusion. | Critical |
| 5.3 | `packages/nexus-ui/src/components/ToolFallback.tsx` | 46-67 | 2.1.1 Keyboard | `ToolFallbackRoot` is a `<div>` with `onClick` but no `tabIndex` or keyboard handler. Although a nested `<button>` (ToolFallbackTrigger) is focusable, clicking the outer div also toggles -- keyboard users cannot trigger from the div. | Major |
| 5.4 | `src/components/layout/Sidebar.tsx` | 207 | 2.1.1 Keyboard | Plugin context menu trigger button uses `opacity-0 group-hover/item:opacity-100` making it invisible to sighted keyboard users navigating without a mouse. Has `focus:opacity-100` which mitigates once focused, but discoverability is poor. | Minor |
| 5.5 | `src/components/layout/Sidebar.tsx` | 440 | 2.1.1 Keyboard | Extension context menu trigger -- same issue as 5.4. | Minor |
| 5.6 | `packages/nexus-ui/src/components/SettingsShell.tsx` | 168 | 2.1.1 Keyboard | Modal backdrop uses `onClick={onClose}` on a `<div>` with no keyboard dismiss support beyond the Escape key handler on `document`. The backdrop itself is not focusable. | Minor |

---

## 6. Step Indicators Without Semantic Structure

Step/wizard/progress indicators that lack proper ARIA roles for navigation landmarks.

| # | File | Line(s) | WCAG Criterion | Description | Severity |
|---|------|---------|----------------|-------------|----------|
| 6.1 | `src/components/permissions/PermissionDialog.tsx` | 87-100 | 1.3.1 Info and Relationships; 4.1.2 Name, Role, Value | Step indicator renders as plain `<div>` elements inside a flex container. Missing: `role="tablist"` on the container, `role="tab"` on each step, `aria-selected` on the active step, and `aria-current="step"`. Screen readers cannot determine the user's position in the workflow. | Major |
| 6.2 | `src/components/marketplace/McpWrapWizard.tsx` | 185-198 | 1.3.1 Info and Relationships; 4.1.2 Name, Role, Value | Five-step wizard indicator uses the same pattern as 6.1 -- plain `<div>` elements with no semantic ARIA roles. | Major |
| 6.3 | `src/components/settings/UpdateCheck.tsx` | 232-243 | 1.3.1 Info and Relationships | Download progress bar is a styled `<div>` with no `role="progressbar"`, `aria-valuenow`, `aria-valuemin`, or `aria-valuemax`. Screen readers cannot determine download progress. | Major |
| 6.4 | `src/components/settings/SystemTab.tsx` | 255-260 | 1.3.1 Info and Relationships | CPU usage progress bar is a styled `<div>` with no `role="progressbar"` or ARIA value attributes. | Major |
| 6.5 | `src/components/settings/SystemTab.tsx` | 270-281 | 1.3.1 Info and Relationships | Memory usage progress bar -- same issue as 6.4. | Major |

---

## 7. Modal Focus Management

Modals and dialogs that may not trap focus or restore focus on close.

| # | File | Line(s) | WCAG Criterion | Description | Severity |
|---|------|---------|----------------|-------------|----------|
| 7.1 | All files using `<Modal>` from `@heroui/react` | Various | 2.4.3 Focus Order | HeroUI's `<Modal>` component generally handles focus trapping and restoration. **No issues found** with the standard HeroUI Modal usage. Focus management appears correct for: `PermissionDialog`, `RuntimeApprovalDialog`, `McpWrapWizard`, `PluginLogs` (Drawer), `KeyChangeWarningDialog`, `PluginViewport` (about/remove modals), `PermissionList` (restore modal), `SecurityTab` (revoke modal), `ExtensionsTab` (remove modal), `McpTab` (regenerate modal), `InstallOverlay`. | N/A |
| 7.2 | `packages/nexus-ui/src/components/SettingsShell.tsx` | 156-186 | 2.4.3 Focus Order; 2.1.2 No Keyboard Trap | The modal variant of `SettingsShell` implements its own overlay with `AnimatePresence`. It does NOT use HeroUI's `<Modal>`. **Focus is not trapped** -- keyboard users can Tab out of the modal into the background content. Focus is not returned to the trigger element on close. | Critical |
| 7.3 | `src/components/InstallOverlay.tsx` | 9-24 | 2.4.3 Focus Order | `InstallOverlay` uses `isDismissable={false}` and `isKeyboardDismissDisabled`, which is correct for a blocking overlay. However, the modal body contains no focusable elements (just a spinner and text), which means keyboard focus has nowhere meaningful to land. | Minor |
| 7.4 | `packages/nexus-ui/src/components/ToolFallback.tsx` | 160-190 | N/A | `ToolFallbackContent` uses `AnimatePresence` for expand/collapse but is inline content, not a modal. No focus trap expected. | N/A |

---

## Summary by Severity

| Severity | Count |
|----------|-------|
| Critical | 7 |
| Major | 33 |
| Minor | 10 |
| N/A (no issue) | 2 |

## Top Priority Fixes

1. **StatusDot accessible names** (2.1) -- Add `aria-label` with the status text (e.g., "Running", "Stopped", "Error") or render a visually hidden `<span>` alongside the dot. This affects every plugin and extension row in the sidebar.

2. **Icon-only buttons** (1.1, 1.2, 1.3, 1.4, 1.5, 1.6-1.8) -- Add `aria-label` to all `isIconOnly` buttons and raw `<button>` elements that contain only icons.

3. **Keyboard-inaccessible divs** (5.1, 5.2) -- Replace `<div onClick>` patterns with `<button>` or add `role="button"`, `tabIndex={0}`, and `onKeyDown` handlers for Enter and Space.

4. **Switch labels** (3.8-3.12) -- Add `aria-label` to all `<Switch>` components. The `PermissionDialog` already does this correctly on line 503 and can serve as a pattern.

5. **Form label associations** (3.1-3.7) -- Use `htmlFor` + `id` on label/input pairs, or use HeroUI's `label` prop on `<Input>` components.

6. **Expandable ARIA states** (4.1-4.6) -- Add `aria-expanded={isOpen}` and `aria-controls={panelId}` to all expand/collapse triggers.

7. **SettingsShell modal focus trap** (7.2) -- Implement focus trapping in the modal variant (e.g., use `react-focus-lock` or switch to HeroUI's `<Modal>`).

8. **Step indicator semantics** (6.1-6.2) -- Add `role="tablist"`, `role="tab"`, `aria-selected`, or alternatively use `role="list"` with `aria-current="step"`.

9. **Progress bar semantics** (6.3-6.5) -- Add `role="progressbar"`, `aria-valuenow`, `aria-valuemin`, `aria-valuemax` to all progress bar `<div>` elements.
