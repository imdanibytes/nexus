import { useState } from "react";
import {
  Button,
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
  Input,
} from "@imdanibytes/nexus-ui";
import { api } from "../lib/api";

export function StorageSection() {
  const [key, setKey] = useState("");
  const [value, setValue] = useState("");
  const [response, setResponse] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function run(fn: () => Promise<unknown>) {
    setLoading(true);
    setResponse(null);
    try {
      const result = await fn();
      setResponse(JSON.stringify(result, null, 2));
    } catch (err) {
      setResponse(`Error: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setLoading(false);
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Key-Value Storage</CardTitle>
        <CardDescription>
          Interactive playground for the plugin storage API (scoped per-plugin)
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          <div className="space-y-1.5">
            <label className="text-sm font-medium">Key</label>
            <Input
              placeholder="my_key"
              value={key}
              onChange={(e) => setKey(e.target.value)}
            />
          </div>
          <div className="space-y-1.5">
            <label className="text-sm font-medium">Value (JSON)</label>
            <textarea
              className="flex min-h-[80px] w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:border-ring disabled:opacity-50 font-mono"
              placeholder='{"example": true}'
              value={value}
              onChange={(e) => setValue(e.target.value)}
            />
          </div>
        </div>

        <div className="flex flex-wrap gap-2">
          <Button
            size="sm"
            disabled={!key || !value || loading}
            onClick={() =>
              run(() => {
                let parsed;
                try {
                  parsed = JSON.parse(value);
                } catch {
                  parsed = value;
                }
                return api(`/api/v1/storage/${encodeURIComponent(key)}`, {
                  method: "PUT",
                  body: JSON.stringify({ value: parsed }),
                });
              })
            }
          >
            Set
          </Button>
          <Button
            size="sm"
            variant="outline"
            disabled={!key || loading}
            onClick={() =>
              run(() => api(`/api/v1/storage/${encodeURIComponent(key)}`))
            }
          >
            Get
          </Button>
          <Button
            size="sm"
            variant="destructive"
            disabled={!key || loading}
            onClick={() =>
              run(() =>
                api(`/api/v1/storage/${encodeURIComponent(key)}`, {
                  method: "DELETE",
                })
              )
            }
          >
            Delete
          </Button>
          <Button
            size="sm"
            variant="secondary"
            disabled={loading}
            onClick={() => run(() => api("/api/v1/storage"))}
          >
            List All Keys
          </Button>
        </div>

        {response !== null && (
          <pre className="rounded-md bg-muted p-3 text-xs font-mono overflow-x-auto max-h-64 overflow-y-auto whitespace-pre-wrap">
            {response}
          </pre>
        )}
      </CardContent>
    </Card>
  );
}
