import { useState } from "react";
import {
  Button,
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
  Input,
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
  Badge,
} from "@imdanibytes/plugin-ui";
import { Send } from "lucide-react";
import { api } from "../lib/api";

export function NetworkSection() {
  const [url, setUrl] = useState("https://httpbin.org/get");
  const [method, setMethod] = useState("GET");
  const [headers, setHeaders] = useState("");
  const [body, setBody] = useState("");
  const [response, setResponse] = useState<{
    status: number;
    headers: Record<string, string>;
    body: string;
  } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function sendRequest() {
    setLoading(true);
    setError(null);
    setResponse(null);

    try {
      let parsedHeaders: Record<string, string> = {};
      if (headers.trim()) {
        parsedHeaders = JSON.parse(headers);
      }

      const payload: Record<string, unknown> = {
        url,
        method,
        headers: parsedHeaders,
      };
      if (body.trim() && method !== "GET") {
        payload.body = body;
      }

      const result = await api<{
        status: number;
        headers: Record<string, string>;
        body: string;
      }>("/api/v1/network/proxy", {
        method: "POST",
        body: JSON.stringify(payload),
      });

      setResponse(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Network Proxy</CardTitle>
        <CardDescription>
          Test outbound requests via POST /api/v1/network/proxy
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex gap-2">
          <Select value={method} onValueChange={setMethod}>
            <SelectTrigger className="w-28">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="GET">GET</SelectItem>
              <SelectItem value="POST">POST</SelectItem>
              <SelectItem value="PUT">PUT</SelectItem>
              <SelectItem value="DELETE">DELETE</SelectItem>
            </SelectContent>
          </Select>
          <Input
            className="flex-1"
            placeholder="https://httpbin.org/get"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
          />
          <Button onClick={sendRequest} disabled={!url || loading}>
            <Send className="size-4" /> Send
          </Button>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          <div className="space-y-1.5">
            <label className="text-sm font-medium">
              Headers <span className="text-muted-foreground font-normal">(JSON, optional)</span>
            </label>
            <textarea
              className="flex min-h-[60px] w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:border-ring font-mono"
              placeholder='{"Accept": "application/json"}'
              value={headers}
              onChange={(e) => setHeaders(e.target.value)}
            />
          </div>
          {method !== "GET" && (
            <div className="space-y-1.5">
              <label className="text-sm font-medium">
                Body <span className="text-muted-foreground font-normal">(optional)</span>
              </label>
              <textarea
                className="flex min-h-[60px] w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:border-ring font-mono"
                placeholder='{"key": "value"}'
                value={body}
                onChange={(e) => setBody(e.target.value)}
              />
            </div>
          )}
        </div>

        {error && (
          <div className="rounded-md bg-destructive/10 border border-destructive/30 p-3 text-sm text-destructive">
            {error}
          </div>
        )}

        {response && (
          <div className="space-y-3">
            <div className="flex items-center gap-2">
              <Badge
                variant={response.status < 400 ? "success" : "error"}
              >
                {response.status}
              </Badge>
              <span className="text-sm text-muted-foreground">Response</span>
            </div>

            {Object.keys(response.headers).length > 0 && (
              <details className="text-sm">
                <summary className="cursor-pointer text-muted-foreground hover:text-foreground">
                  Response Headers ({Object.keys(response.headers).length})
                </summary>
                <pre className="mt-1 rounded-md bg-muted p-2 text-xs font-mono overflow-x-auto">
                  {JSON.stringify(response.headers, null, 2)}
                </pre>
              </details>
            )}

            <pre className="rounded-md bg-muted p-3 text-xs font-mono overflow-x-auto max-h-80 overflow-y-auto whitespace-pre-wrap">
              {(() => {
                try {
                  return JSON.stringify(JSON.parse(response.body), null, 2);
                } catch {
                  return response.body;
                }
              })()}
            </pre>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
