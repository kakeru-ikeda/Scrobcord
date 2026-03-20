import { useState } from "react";
import Dashboard from "./pages/Dashboard";
import Settings from "./pages/Settings";

type Page = "dashboard" | "settings";

function App() {
  const [page, setPage] = useState<Page>("dashboard");

  return (
    <div className="h-screen w-screen overflow-hidden bg-background">
      {page === "dashboard" ? (
        <Dashboard onNavigateSettings={() => setPage("settings")} />
      ) : (
        <Settings onBack={() => setPage("dashboard")} />
      )}
    </div>
  );
}

export default App;
