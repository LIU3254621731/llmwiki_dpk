import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "./App";
import "./index.css";

// Initialize theme from localStorage before first render
const storedTheme = localStorage.getItem("llmwiki-theme");
if (storedTheme === "dark") {
  document.documentElement.classList.add("dark");
} else if (!storedTheme && window.matchMedia?.("(prefers-color-scheme: dark)").matches) {
  document.documentElement.classList.add("dark");
}

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { staleTime: 30_000, retry: 1 },
  },
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </React.StrictMode>
);
