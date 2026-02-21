export interface AuditLogRow {
  id: number;
  timestamp: string;
  actor: string;
  source_id: string | null;
  severity: string;
  action: string;
  subject: string | null;
  result: string;
  details: Record<string, unknown> | null;
}
