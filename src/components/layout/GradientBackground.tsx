/**
 * Animated gradient background — GPU-friendly.
 *
 * Each blob is a radial-gradient div (the gradient provides natural softness —
 * no CSS `filter: blur()` needed). Blobs are layer-promoted via
 * `will-change: transform` and animated exclusively with `transform: translate3d()`.
 * The GPU rasterizes each gradient texture once on mount, then composites the
 * cached layers at new positions each frame — no Gaussian blur recomputation.
 */

const BLOBS = [
  {
    // Teal — upper left, slow drift
    size: 800,
    x: "10%",
    y: "10%",
    color: "rgba(6,182,212,0.15)",
    animation: "drift-1 25s ease-in-out infinite alternate",
  },
  {
    // Violet — upper right, medium drift
    size: 700,
    x: "75%",
    y: "15%",
    color: "rgba(139,92,246,0.12)",
    animation: "drift-2 30s ease-in-out infinite alternate",
  },
  {
    // Pink — lower left
    size: 750,
    x: "15%",
    y: "70%",
    color: "rgba(236,72,153,0.10)",
    animation: "drift-3 28s ease-in-out infinite alternate",
  },
  {
    // Blue — lower right
    size: 650,
    x: "70%",
    y: "75%",
    color: "rgba(59,130,246,0.10)",
    animation: "drift-4 22s ease-in-out infinite alternate",
  },
  {
    // Amber — top right accent
    size: 600,
    x: "80%",
    y: "5%",
    color: "rgba(245,158,11,0.08)",
    animation: "drift-5 26s ease-in-out infinite alternate",
  },
] as const;

export function GradientBackground() {
  return (
    <div className="fixed inset-0 -z-10 overflow-hidden bg-background">
      <style>{keyframes}</style>
      {BLOBS.map((blob, i) => (
        <div
          key={i}
          style={{
            position: "absolute",
            left: blob.x,
            top: blob.y,
            width: blob.size,
            height: blob.size,
            borderRadius: "50%",
            background: `radial-gradient(circle, ${blob.color} 0%, transparent 70%)`,
            willChange: "transform",
            animation: blob.animation,
            // Nudge off main thread — transform-only animations stay on compositor
            transform: "translate3d(0,0,0)",
          }}
        />
      ))}
    </div>
  );
}

// Each keyframe uses ONLY translate3d — no layout, no paint, no re-rasterization.
const keyframes = `
@keyframes drift-1 {
  0%   { transform: translate3d(0, 0, 0); }
  100% { transform: translate3d(180px, 120px, 0); }
}
@keyframes drift-2 {
  0%   { transform: translate3d(0, 0, 0); }
  100% { transform: translate3d(-160px, 100px, 0); }
}
@keyframes drift-3 {
  0%   { transform: translate3d(0, 0, 0); }
  100% { transform: translate3d(140px, -160px, 0); }
}
@keyframes drift-4 {
  0%   { transform: translate3d(0, 0, 0); }
  100% { transform: translate3d(-180px, -120px, 0); }
}
@keyframes drift-5 {
  0%   { transform: translate3d(0, 0, 0); }
  100% { transform: translate3d(-130px, 150px, 0); }
}
`;
