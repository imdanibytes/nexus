import { Component, type ErrorInfo, type ReactNode } from "react";
import { AlertTriangle } from "lucide-react";

interface Props {
  children: ReactNode;
  /** Compact inline fallback instead of full-width card */
  inline?: boolean;
  /** Label shown in the fallback (e.g. plugin name) */
  label?: string;
}

interface State {
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("[ErrorBoundary]", this.props.label ?? "", error, info);
  }

  render() {
    if (!this.state.error) {
      return this.props.children;
    }

    const message = this.state.error.message || "Something went wrong";

    if (this.props.inline) {
      return (
        <div className="flex items-center gap-2 px-3 py-2 text-[11px] text-nx-error bg-nx-error-muted rounded-[var(--radius-button)]">
          <AlertTriangle size={12} strokeWidth={2} className="flex-shrink-0" />
          <span className="truncate">
            {this.props.label ? `${this.props.label}: ` : ""}
            {message}
          </span>
        </div>
      );
    }

    return (
      <div className="rounded-[var(--radius-card)] border border-nx-error/30 bg-nx-error-muted/30 p-5">
        <div className="flex items-center gap-2 mb-2">
          <AlertTriangle size={15} strokeWidth={1.5} className="text-nx-error" />
          <h3 className="text-[13px] font-medium text-nx-error">
            {this.props.label ? `${this.props.label} — ` : ""}Render Error
          </h3>
        </div>
        <p className="text-[12px] text-nx-text-muted font-mono break-all">
          {message}
        </p>
        <button
          onClick={() => this.setState({ error: null })}
          className="mt-3 px-3 py-1.5 text-[11px] font-medium bg-nx-overlay text-nx-text rounded-[var(--radius-button)] hover:bg-nx-wash transition-colors duration-150"
        >
          Retry
        </button>
      </div>
    );
  }
}
