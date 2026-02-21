import { useState, useEffect } from "react";
import {
  Button,
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
  CardAction,
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
  Switch,
  Skeleton,
  Badge,
} from "@imdanibytes/nexus-ui";
import { RefreshCw, Save } from "lucide-react";
import { toast } from "sonner";
import { api } from "../lib/api";

// Matches the settings declared in plugin.json
const SETTING_DEFS = [
  {
    key: "accent_color",
    type: "select",
    label: "Accent Color",
    description: "Color accent used in the playground UI",
    default: "teal",
    options: ["teal", "amber", "blue"],
  },
  {
    key: "show_code_snippets",
    type: "boolean",
    label: "Show Code Snippets",
    description: "Display import snippets alongside component demos",
    default: true,
  },
] as const;

export function SettingsSection() {
  const [settings, setSettings] = useState<Record<string, unknown> | null>(
    null
  );
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  async function fetchSettings() {
    setLoading(true);
    setError(null);
    try {
      const data = await api<Record<string, unknown>>("/api/v1/settings");
      setSettings(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  async function saveSettings() {
    if (!settings) return;
    setSaving(true);
    try {
      await api("/api/v1/settings", {
        method: "PUT",
        body: JSON.stringify(settings),
      });
      toast.success("Settings saved");
    } catch (err) {
      toast.error(
        `Failed to save: ${err instanceof Error ? err.message : String(err)}`
      );
    } finally {
      setSaving(false);
    }
  }

  useEffect(() => {
    fetchSettings();
  }, []);

  function updateSetting(key: string, value: unknown) {
    setSettings((prev) => (prev ? { ...prev, [key]: value } : prev));
  }

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader>
          <CardTitle>Plugin Settings</CardTitle>
          <CardDescription>
            Read with GET /api/v1/settings, write with PUT /api/v1/settings
          </CardDescription>
          <CardAction>
            <div className="flex gap-1.5">
              <Button
                variant="outline"
                size="icon-sm"
                onClick={fetchSettings}
                disabled={loading}
              >
                <RefreshCw
                  className={`size-4 ${loading ? "animate-spin" : ""}`}
                />
              </Button>
              <Button
                size="sm"
                onClick={saveSettings}
                disabled={saving || !settings}
              >
                <Save className="size-3.5" /> Save
              </Button>
            </div>
          </CardAction>
        </CardHeader>
        <CardContent>
          {error && (
            <div className="rounded-md bg-destructive/10 border border-destructive/30 p-3 text-sm text-destructive mb-4">
              {error}
            </div>
          )}

          {loading && !settings && (
            <div className="space-y-4">
              {Array.from({ length: 2 }).map((_, i) => (
                <div key={i} className="flex justify-between items-center">
                  <Skeleton className="h-4 w-32" />
                  <Skeleton className="h-9 w-28" />
                </div>
              ))}
            </div>
          )}

          {settings && (
            <div className="space-y-5">
              {SETTING_DEFS.map((def) => (
                <div
                  key={def.key}
                  className="flex items-center justify-between gap-4"
                >
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium">{def.label}</span>
                      <Badge variant="secondary">{def.type}</Badge>
                    </div>
                    <p className="text-xs text-muted-foreground mt-0.5">
                      {def.description}
                    </p>
                  </div>

                  {def.type === "select" && (
                    <Select
                      value={String(settings[def.key] ?? def.default)}
                      // eslint-disable-next-line react-perf/jsx-no-new-function-as-prop
                      onValueChange={(v) => updateSetting(def.key, v)}
                    >
                      <SelectTrigger className="w-28">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {def.options.map((opt) => (
                          <SelectItem key={opt} value={opt}>
                            {opt}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  )}

                  {def.type === "boolean" && (
                    <Switch
                      checked={
                        (settings[def.key] as boolean) ?? def.default
                      }
                      // eslint-disable-next-line react-perf/jsx-no-new-function-as-prop
                      onCheckedChange={(v) => updateSetting(def.key, v)}
                    />
                  )}
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Manifest Declaration</CardTitle>
          <CardDescription>
            How these settings are declared in plugin.json
          </CardDescription>
        </CardHeader>
        <CardContent>
          <pre className="rounded-md bg-muted p-3 text-xs font-mono overflow-x-auto whitespace-pre-wrap">
            {JSON.stringify(SETTING_DEFS, null, 2)}
          </pre>
        </CardContent>
      </Card>

      {settings && (
        <Card>
          <CardHeader>
            <CardTitle>Raw Values</CardTitle>
            <CardDescription>Current settings as returned by the API</CardDescription>
          </CardHeader>
          <CardContent>
            <pre className="rounded-md bg-muted p-3 text-xs font-mono overflow-x-auto whitespace-pre-wrap">
              {JSON.stringify(settings, null, 2)}
            </pre>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
