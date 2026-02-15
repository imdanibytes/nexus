import { useState, useEffect } from "react";
import {
  Button,
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
  CardAction,
  Skeleton,
} from "@imdanibytes/plugin-ui";
import { RefreshCw } from "lucide-react";
import { api } from "../lib/api";

interface SystemInfo {
  os: string;
  os_version: string;
  hostname: string;
  uptime: number;
  cpu_count: number;
  total_memory: number;
  nexus_version: string;
}

function formatUptime(seconds: number): string {
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  return d > 0 ? `${d}d ${h}h ${m}m` : `${h}h ${m}m`;
}

function formatBytes(bytes: number): string {
  const gb = (bytes / (1024 * 1024 * 1024)).toFixed(1);
  return `${gb} GB`;
}

export function SystemSection() {
  const [info, setInfo] = useState<SystemInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  async function fetchInfo() {
    setLoading(true);
    setError(null);
    try {
      const data = await api<SystemInfo>("/api/v1/system/info");
      setInfo(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    fetchInfo();
  }, []);

  const rows: [string, string][] = info
    ? [
        ["OS", `${info.os} ${info.os_version}`],
        ["Hostname", info.hostname],
        ["Uptime", formatUptime(info.uptime)],
        ["CPUs", String(info.cpu_count)],
        ["Memory", formatBytes(info.total_memory)],
        ["Nexus Version", `v${info.nexus_version}`],
      ]
    : [];

  return (
    <Card>
      <CardHeader>
        <CardTitle>System Info</CardTitle>
        <CardDescription>
          Live data from GET /api/v1/system/info
        </CardDescription>
        <CardAction>
          <Button
            variant="outline"
            size="icon-sm"
            onClick={fetchInfo}
            disabled={loading}
          >
            <RefreshCw className={`size-4 ${loading ? "animate-spin" : ""}`} />
          </Button>
        </CardAction>
      </CardHeader>
      <CardContent>
        {error && (
          <div className="rounded-md bg-destructive/10 border border-destructive/30 p-3 text-sm text-destructive">
            {error}
          </div>
        )}

        {loading && !info && (
          <div className="space-y-3">
            {Array.from({ length: 6 }).map((_, i) => (
              <div key={i} className="flex justify-between">
                <Skeleton className="h-4 w-20" />
                <Skeleton className="h-4 w-32" />
              </div>
            ))}
          </div>
        )}

        {info && (
          <div className="divide-y divide-border">
            {rows.map(([label, value]) => (
              <div
                key={label}
                className="flex justify-between py-2.5 text-sm"
              >
                <span className="text-muted-foreground">{label}</span>
                <span className="font-medium font-mono text-foreground">
                  {value}
                </span>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
