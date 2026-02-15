import type { ReactNode } from "react";
import { AppSidebar } from "./Sidebar";
import { RuntimeApprovalDialog } from "../permissions/RuntimeApprovalDialog";
import { ErrorBoundary } from "../ErrorBoundary";
import { Toaster } from "@/components/ui/sonner";
import { SidebarProvider, SidebarInset } from "@/components/ui/sidebar";

export function Shell({ children }: { children: ReactNode }) {
  return (
    <SidebarProvider defaultOpen={true} className="bg-nx-deep">
      <ErrorBoundary inline label="Sidebar">
        <AppSidebar />
      </ErrorBoundary>
      <SidebarInset className="relative overflow-hidden bg-nx-base">
        {children}
      </SidebarInset>

      <ErrorBoundary inline label="Approval Dialog">
        <RuntimeApprovalDialog />
      </ErrorBoundary>

      <Toaster position="bottom-right" />
    </SidebarProvider>
  );
}
