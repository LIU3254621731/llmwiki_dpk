import { Component, type ReactNode } from "react";

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false, error: null };

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) return this.props.fallback;
      return (
        <div className="flex items-center justify-center h-full min-h-[400px] bg-slate-50">
          <div className="max-w-md text-center">
            <div className="text-4xl mb-4">⚠</div>
            <h1 className="text-lg font-semibold text-slate-800 mb-2">页面出现错误</h1>
            <p className="text-sm text-slate-500 mb-4">
              {this.state.error?.message || "渲染组件时发生未知错误"}
            </p>
            <button
              onClick={() => {
                this.setState({ hasError: false, error: null });
                window.location.reload();
              }}
              className="px-4 py-2 bg-brand-600 text-white rounded text-sm hover:bg-brand-700"
            >
              重新加载
            </button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}
