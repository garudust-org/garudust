import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
}
interface State {
  error: Error | null;
}

// Catches render errors in a page so one bad page shows a message instead of
// blanking the whole window (a webview render throw otherwise leaves a black screen).
export default class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("Page error:", error, info);
  }

  reset = () => this.setState({ error: null });

  render() {
    if (this.state.error) {
      return (
        <div className="flex h-full flex-col items-center justify-center gap-3 p-8 text-center">
          <div className="text-lg text-red-400">Something went wrong on this page</div>
          <pre className="max-w-lg overflow-auto rounded-lg border border-neutral-800 bg-neutral-900 p-3 text-left text-xs text-neutral-400">
            {this.state.error.message}
          </pre>
          <button
            className="rounded-lg bg-amber-500 px-4 py-2 text-sm font-medium text-neutral-950"
            onClick={this.reset}
          >
            Try again
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
