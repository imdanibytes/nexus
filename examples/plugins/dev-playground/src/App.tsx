import { useState, useEffect } from "react";
import { NexusProvider } from "@imdanibytes/plugin-ui";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@imdanibytes/plugin-ui";
import { ComponentsSection } from "./sections/ComponentsSection";
import { StorageSection } from "./sections/StorageSection";
import { SystemSection } from "./sections/SystemSection";
import { NetworkSection } from "./sections/NetworkSection";
import { SettingsSection } from "./sections/SettingsSection";
import { McpSection } from "./sections/McpSection";

export function App() {
  const [apiUrl, setApiUrl] = useState<string | null>(null);

  useEffect(() => {
    fetch("/api/config")
      .then((r) => r.json())
      .then((cfg) => setApiUrl(cfg.apiUrl))
      .catch(() => setApiUrl("http://localhost:9600"));
  }, []);

  if (!apiUrl) return null;

  return (
    <NexusProvider apiUrl={apiUrl}>
      <div className="min-h-screen bg-background text-foreground p-6">
        <div className="mb-6">
          <h1 className="text-xl font-bold">Dev Playground</h1>
          <p className="text-sm text-muted-foreground">
            Interactive reference for Nexus plugin development
          </p>
        </div>

        <Tabs defaultValue="components">
          <TabsList variant="line">
            <TabsTrigger value="components">Components</TabsTrigger>
            <TabsTrigger value="storage">Storage</TabsTrigger>
            <TabsTrigger value="system">System</TabsTrigger>
            <TabsTrigger value="network">Network</TabsTrigger>
            <TabsTrigger value="settings">Settings</TabsTrigger>
            <TabsTrigger value="mcp">MCP</TabsTrigger>
          </TabsList>

          <TabsContent value="components" className="mt-4">
            <ComponentsSection />
          </TabsContent>
          <TabsContent value="storage" className="mt-4">
            <StorageSection />
          </TabsContent>
          <TabsContent value="system" className="mt-4">
            <SystemSection />
          </TabsContent>
          <TabsContent value="network" className="mt-4">
            <NetworkSection />
          </TabsContent>
          <TabsContent value="settings" className="mt-4">
            <SettingsSection />
          </TabsContent>
          <TabsContent value="mcp" className="mt-4">
            <McpSection />
          </TabsContent>
        </Tabs>
      </div>
    </NexusProvider>
  );
}
