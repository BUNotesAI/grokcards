import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import AppShell from "@/components/AppShell";
import TodoPage from "@/pages/TodoPage";
import PlaceholderPage from "@/pages/PlaceholderPage";
import { FileText, Bookmark, Code2, Settings } from "lucide-react";

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route element={<AppShell />}>
          <Route path="/todo" element={<TodoPage />} />
          <Route
            path="/notes"
            element={
              <PlaceholderPage
                icon={FileText}
                title="Notes"
                description="Quick notes and scratch pad. Markdown support, local-first storage."
              />
            }
          />
          <Route
            path="/bookmarks"
            element={
              <PlaceholderPage
                icon={Bookmark}
                title="Bookmarks"
                description="Save and organize links. Tag-based filtering, full-text search."
              />
            }
          />
          <Route
            path="/snippets"
            element={
              <PlaceholderPage
                icon={Code2}
                title="Snippets"
                description="Code snippets library. Syntax highlighting, quick copy, language tags."
              />
            }
          />
          <Route
            path="/settings"
            element={
              <PlaceholderPage
                icon={Settings}
                title="Settings"
                description="App preferences, theme, data management, keyboard shortcuts."
              />
            }
          />
          <Route path="*" element={<Navigate to="/todo" replace />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}

export default App;
