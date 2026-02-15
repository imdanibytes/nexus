import {
  Button,
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
  CardFooter,
  CardAction,
  Input,
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
  Switch,
  Dialog,
  DialogTrigger,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
  Badge,
  Tabs,
  TabsList,
  TabsTrigger,
  TabsContent,
  Separator,
  Skeleton,
  Tooltip,
  TooltipTrigger,
  TooltipContent,
  toast,
} from "@imdanibytes/nexus-ui";
import {
  Heart,
  Download,
  Trash2,
  Settings,
  Mail,
  Search,
} from "lucide-react";

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="mb-8">
      <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-3">
        {title}
      </h3>
      {children}
    </div>
  );
}

export function ComponentsSection() {
  return (
    <div className="space-y-6">
      {/* ── Buttons ───────────────────────────────────────── */}
      <Section title="Buttons">
        <div className="space-y-4">
          <div className="flex flex-wrap items-center gap-2">
            <Button>Default</Button>
            <Button variant="destructive">Destructive</Button>
            <Button variant="outline">Outline</Button>
            <Button variant="secondary">Secondary</Button>
            <Button variant="ghost">Ghost</Button>
            <Button variant="link">Link</Button>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Button size="xs">Extra Small</Button>
            <Button size="sm">Small</Button>
            <Button size="default">Default</Button>
            <Button size="lg">Large</Button>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Button disabled>Disabled</Button>
            <Button size="icon"><Heart className="size-4" /></Button>
            <Button size="icon-xs"><Search className="size-3" /></Button>
            <Button size="icon-sm"><Settings className="size-4" /></Button>
            <Button size="icon-lg"><Download className="size-4" /></Button>
            <Button variant="destructive" size="sm">
              <Trash2 className="size-3.5" /> Delete
            </Button>
            <Button variant="outline">
              <Mail className="size-4" /> Send Email
            </Button>
          </div>
        </div>
      </Section>

      {/* ── Cards ─────────────────────────────────────────── */}
      <Section title="Cards">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <Card>
            <CardHeader>
              <CardTitle>Basic Card</CardTitle>
              <CardDescription>A simple card with title and description</CardDescription>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-muted-foreground">
                Card content goes here. Use cards to group related information.
              </p>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Card with Action</CardTitle>
              <CardDescription>Has a header action button</CardDescription>
              <CardAction>
                <Button variant="outline" size="sm">Edit</Button>
              </CardAction>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-muted-foreground">
                The CardAction component places a button in the top-right of the header.
              </p>
            </CardContent>
            <CardFooter>
              <Button variant="ghost" size="sm">Cancel</Button>
              <Button size="sm" className="ml-auto">Save</Button>
            </CardFooter>
          </Card>
        </div>
      </Section>

      {/* ── Forms ─────────────────────────────────────────── */}
      <Section title="Forms">
        <Card>
          <CardContent className="space-y-4">
            <div className="space-y-1.5">
              <label className="text-sm font-medium">Input</label>
              <Input placeholder="Type something..." />
            </div>
            <div className="space-y-1.5">
              <label className="text-sm font-medium">Disabled Input</label>
              <Input placeholder="Can't touch this" disabled />
            </div>
            <div className="space-y-1.5">
              <label className="text-sm font-medium">Select</label>
              <Select>
                <SelectTrigger>
                  <SelectValue placeholder="Choose an option" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="react">React</SelectItem>
                  <SelectItem value="vue">Vue</SelectItem>
                  <SelectItem value="svelte">Svelte</SelectItem>
                  <SelectItem value="solid">Solid</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="flex items-center justify-between">
              <div>
                <label className="text-sm font-medium">Switch (default)</label>
                <p className="text-xs text-muted-foreground">Toggle something on or off</p>
              </div>
              <Switch />
            </div>
            <div className="flex items-center justify-between">
              <div>
                <label className="text-sm font-medium">Switch (small)</label>
                <p className="text-xs text-muted-foreground">Compact variant</p>
              </div>
              <Switch size="sm" />
            </div>
          </CardContent>
        </Card>
      </Section>

      {/* ── Feedback ──────────────────────────────────────── */}
      <Section title="Feedback">
        <div className="space-y-4">
          <div className="flex flex-wrap gap-2">
            <Dialog>
              <DialogTrigger asChild>
                <Button variant="outline">Open Dialog</Button>
              </DialogTrigger>
              <DialogContent>
                <DialogHeader>
                  <DialogTitle>Dialog Title</DialogTitle>
                  <DialogDescription>
                    This is a dialog component. Use it for confirmations, forms,
                    or any content that needs focused attention.
                  </DialogDescription>
                </DialogHeader>
                <DialogFooter showCloseButton>
                  <Button>Confirm</Button>
                </DialogFooter>
              </DialogContent>
            </Dialog>

            <Button variant="outline" onClick={() => toast.success("Operation completed")}>
              Toast: Success
            </Button>
            <Button variant="outline" onClick={() => toast.error("Something went wrong")}>
              Toast: Error
            </Button>
            <Button variant="outline" onClick={() => toast.warning("Proceed with caution")}>
              Toast: Warning
            </Button>
            <Button variant="outline" onClick={() => toast.info("Here's some info")}>
              Toast: Info
            </Button>
          </div>

          <div className="flex flex-wrap gap-2">
            <Badge>Default</Badge>
            <Badge variant="secondary">Secondary</Badge>
            <Badge variant="destructive">Destructive</Badge>
            <Badge variant="outline">Outline</Badge>
            <Badge variant="success">Success</Badge>
            <Badge variant="warning">Warning</Badge>
            <Badge variant="error">Error</Badge>
            <Badge variant="info">Info</Badge>
            <Badge variant="accent">Accent</Badge>
            <Badge variant="highlight">Highlight</Badge>
          </div>
        </div>
      </Section>

      {/* ── Layout ────────────────────────────────────────── */}
      <Section title="Layout">
        <Card>
          <CardContent className="space-y-4">
            <div>
              <p className="text-sm font-medium mb-2">Tabs (default variant)</p>
              <Tabs defaultValue="tab1">
                <TabsList>
                  <TabsTrigger value="tab1">Tab 1</TabsTrigger>
                  <TabsTrigger value="tab2">Tab 2</TabsTrigger>
                  <TabsTrigger value="tab3">Tab 3</TabsTrigger>
                </TabsList>
                <TabsContent value="tab1" className="p-3 text-sm text-muted-foreground">
                  Content for tab 1
                </TabsContent>
                <TabsContent value="tab2" className="p-3 text-sm text-muted-foreground">
                  Content for tab 2
                </TabsContent>
                <TabsContent value="tab3" className="p-3 text-sm text-muted-foreground">
                  Content for tab 3
                </TabsContent>
              </Tabs>
            </div>

            <Separator />

            <div>
              <p className="text-sm font-medium mb-2">Tabs (line variant)</p>
              <Tabs defaultValue="a">
                <TabsList variant="line">
                  <TabsTrigger value="a">Alpha</TabsTrigger>
                  <TabsTrigger value="b">Beta</TabsTrigger>
                  <TabsTrigger value="c">Gamma</TabsTrigger>
                </TabsList>
                <TabsContent value="a" className="p-3 text-sm text-muted-foreground">
                  Alpha content
                </TabsContent>
                <TabsContent value="b" className="p-3 text-sm text-muted-foreground">
                  Beta content
                </TabsContent>
                <TabsContent value="c" className="p-3 text-sm text-muted-foreground">
                  Gamma content
                </TabsContent>
              </Tabs>
            </div>

            <Separator />

            <div>
              <p className="text-sm font-medium mb-2">Separator</p>
              <div className="flex items-center gap-3 h-5">
                <span className="text-sm text-muted-foreground">Left</span>
                <Separator orientation="vertical" />
                <span className="text-sm text-muted-foreground">Center</span>
                <Separator orientation="vertical" />
                <span className="text-sm text-muted-foreground">Right</span>
              </div>
            </div>

            <Separator />

            <div>
              <p className="text-sm font-medium mb-2">Skeleton loading</p>
              <div className="space-y-2">
                <Skeleton className="h-4 w-3/4" />
                <Skeleton className="h-4 w-1/2" />
                <Skeleton className="h-10 w-full" />
              </div>
            </div>
          </CardContent>
        </Card>
      </Section>

      {/* ── Tooltips ──────────────────────────────────────── */}
      <Section title="Tooltips">
        <div className="flex flex-wrap gap-3">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="outline">Top</Button>
            </TooltipTrigger>
            <TooltipContent side="top">Tooltip on top</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="outline">Right</Button>
            </TooltipTrigger>
            <TooltipContent side="right">Tooltip on right</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="outline">Bottom</Button>
            </TooltipTrigger>
            <TooltipContent side="bottom">Tooltip on bottom</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="outline">Left</Button>
            </TooltipTrigger>
            <TooltipContent side="left">Tooltip on left</TooltipContent>
          </Tooltip>
        </div>
      </Section>
    </div>
  );
}
