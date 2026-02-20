/**
 * Gradient background — static version.
 * The animated blur blobs (5x 900px @ blur-160px) were causing 1100ms+ GPU compositor
 * stalls on every interaction. CSS blur on large animated elements creates massive
 * GPU textures that must be re-composited on every frame.
 *
 * TODO: Bring back animation using a single pre-rendered canvas/SVG with
 * will-change:transform (no per-frame blur), or a CSS conic-gradient approach.
 */
export function GradientBackground() {
  return (
    <div className="fixed inset-0 -z-10 overflow-hidden bg-background">
      {/* Single static gradient — no blur, no animation, no GPU stall */}
      <div
        className="absolute inset-0"
        style={{
          background: `
            radial-gradient(ellipse 80% 60% at 15% 20%, rgba(6,182,212,0.15) 0%, transparent 70%),
            radial-gradient(ellipse 70% 55% at 85% 25%, rgba(139,92,246,0.12) 0%, transparent 70%),
            radial-gradient(ellipse 75% 60% at 20% 85%, rgba(236,72,153,0.10) 0%, transparent 70%),
            radial-gradient(ellipse 65% 50% at 80% 80%, rgba(59,130,246,0.10) 0%, transparent 70%),
            radial-gradient(ellipse 60% 45% at 85% 10%, rgba(245,158,11,0.08) 0%, transparent 70%)
          `,
        }}
      />
    </div>
  );
}
