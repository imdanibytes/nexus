import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
  Badge,
  Separator,
} from "@imdanibytes/plugin-ui";
import manifest from "../../plugin.json";

interface McpTool {
  name: string;
  description: string;
  permissions: string[];
  input_schema: {
    type: string;
    properties: Record<string, unknown>;
    required: string[];
  };
}

const tools: McpTool[] = manifest.mcp.tools;

export function McpSection() {
  return (
    <div className="space-y-4">
      <Card>
        <CardHeader>
          <CardTitle>MCP Tools</CardTitle>
          <CardDescription>
            Tools declared in plugin.json and handled by server.js via POST /mcp/call
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {tools.map((tool, i) => (
            <div key={tool.name}>
              {i > 0 && <Separator className="mb-4" />}
              <div className="space-y-2">
                <div className="flex items-center gap-2">
                  <code className="text-sm font-mono font-semibold text-foreground">
                    {tool.name}
                  </code>
                  {tool.permissions.map((perm) => (
                    <Badge key={perm} variant="accent">
                      {perm}
                    </Badge>
                  ))}
                  {tool.permissions.length === 0 && (
                    <Badge variant="secondary">no permissions</Badge>
                  )}
                </div>
                <p className="text-sm text-muted-foreground">
                  {tool.description}
                </p>
                <div>
                  <span className="text-xs font-medium text-muted-foreground">
                    Input Schema
                  </span>
                  <pre className="mt-1 rounded-md bg-muted p-2 text-xs font-mono overflow-x-auto">
                    {JSON.stringify(tool.input_schema, null, 2)}
                  </pre>
                </div>
              </div>
            </div>
          ))}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Request Format</CardTitle>
          <CardDescription>
            How the MCP gateway calls your plugin's tools
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-3">
            <div>
              <span className="text-xs font-medium text-muted-foreground">
                Request — POST /mcp/call
              </span>
              <pre className="mt-1 rounded-md bg-muted p-3 text-xs font-mono overflow-x-auto whitespace-pre-wrap">
                {JSON.stringify(
                  {
                    tool_name: "get_system_info",
                    arguments: {},
                  },
                  null,
                  2
                )}
              </pre>
            </div>
            <div>
              <span className="text-xs font-medium text-muted-foreground">
                Response
              </span>
              <pre className="mt-1 rounded-md bg-muted p-3 text-xs font-mono overflow-x-auto whitespace-pre-wrap">
                {JSON.stringify(
                  {
                    content: [
                      {
                        type: "text",
                        text: '{"os":"macOS","hostname":"my-mac",...}',
                      },
                    ],
                    is_error: false,
                  },
                  null,
                  2
                )}
              </pre>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
