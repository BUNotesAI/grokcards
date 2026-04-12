import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "./App";
import {
  perfLog,
  startHeartbeat,
  startLongTaskObserver,
  startErrorCapture,
} from "@/lib/perf";
import "./index.css";

perfLog("main.tsx loaded — JS bundle parsed");
startErrorCapture();
startHeartbeat();
startLongTaskObserver();

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      // 画布场景：数据变化少，长缓存 + 手动 invalidate
      staleTime: 5 * 60 * 1000,
      retry: 1,
    },
  },
});

perfLog("ReactDOM.createRoot — about to render");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </React.StrictMode>,
);
