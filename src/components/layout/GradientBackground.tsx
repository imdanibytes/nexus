/**
 * Animated metaball-style gradient background.
 * Large colored blobs drift slowly behind frosted-glass panels.
 * Opacity adapts to light/dark mode.
 */
export function GradientBackground() {
  return (
    <div className="fixed inset-0 -z-10 overflow-hidden bg-background">
      {/* Blob 1 — teal/cyan — top-left corner */}
      <div
        className="absolute w-[900px] h-[900px] rounded-full blur-[160px] animate-[drift1_25s_ease-in-out_infinite] opacity-20 dark:opacity-60"
        style={{ background: "radial-gradient(circle, #06b6d4, transparent 70%)", top: "-30%", left: "-25%" }}
      />
      {/* Blob 2 — violet/purple — right edge */}
      <div
        className="absolute w-[800px] h-[800px] rounded-full blur-[160px] animate-[drift2_30s_ease-in-out_infinite] opacity-15 dark:opacity-50"
        style={{ background: "radial-gradient(circle, #8b5cf6, transparent 70%)", top: "15%", right: "-30%" }}
      />
      {/* Blob 3 — pink/rose — bottom-left */}
      <div
        className="absolute w-[850px] h-[850px] rounded-full blur-[160px] animate-[drift3_35s_ease-in-out_infinite] opacity-15 dark:opacity-40"
        style={{ background: "radial-gradient(circle, #ec4899, transparent 70%)", bottom: "-35%", left: "-15%" }}
      />
      {/* Blob 4 — blue — bottom-right corner */}
      <div
        className="absolute w-[750px] h-[750px] rounded-full blur-[160px] animate-[drift4_28s_ease-in-out_infinite] opacity-15 dark:opacity-45"
        style={{ background: "radial-gradient(circle, #3b82f6, transparent 70%)", bottom: "-25%", right: "-20%" }}
      />
      {/* Blob 5 — amber — top-right */}
      <div
        className="absolute w-[700px] h-[700px] rounded-full blur-[160px] animate-[drift1_32s_ease-in-out_infinite_reverse] opacity-10 dark:opacity-30"
        style={{ background: "radial-gradient(circle, #f59e0b, transparent 70%)", top: "-25%", right: "-10%" }}
      />
    </div>
  );
}
