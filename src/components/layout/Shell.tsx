import { type ReactNode, useEffect } from "react";
import { AppSidebar } from "./Sidebar";
import { GradientBackground } from "./GradientBackground";
import { RuntimeApprovalDialog } from "../permissions/RuntimeApprovalDialog";
import { ErrorBoundary } from "../ErrorBoundary";
import { Toaster } from "@imdanibytes/nexus-ui";

/**
 * Keep the JS thread alive so WebKit doesn't throttle React's scheduler.
 * Without this, setTimeout/MessageChannel callbacks (which React uses
 * internally) get delayed 3-7s because the only animations are GPU-
 * composited gradient blobs that produce zero JS wakeups.
 */
function useRafHeartbeat() {
  useEffect(() => {
    let id: number;
    const tick = () => { id = requestAnimationFrame(tick); };
    id = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(id);
  }, []);
}

export function Shell({ children }: { children: ReactNode }) {
  useRafHeartbeat();
  return (
    <div className="relative flex h-screen text-foreground">
      <GradientBackground />

      <ErrorBoundary inline label="Sidebar">
        <AppSidebar />
      </ErrorBoundary>

      <main className="relative flex-1 overflow-hidden backdrop-blur-2xl bg-background/40 border-l border-default-200/50">
        {children}
      </main>

      <ErrorBoundary inline label="Approval Dialog">
        <RuntimeApprovalDialog />
      </ErrorBoundary>

      <Toaster position="bottom-right" />
    </div>
  );
}
