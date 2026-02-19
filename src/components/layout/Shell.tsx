import type { ReactNode } from "react";
import { AppSidebar } from "./Sidebar";
import { GradientBackground } from "./GradientBackground";
import { RuntimeApprovalDialog } from "../permissions/RuntimeApprovalDialog";
import { ErrorBoundary } from "../ErrorBoundary";
import { Toaster } from "@imdanibytes/nexus-ui";

export function Shell({ children }: { children: ReactNode }) {
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
