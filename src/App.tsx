import { useState } from "react";
import Dashboard from "./pages/Dashboard";
import Settings from "./pages/Settings";
import { useConnectionStatus } from "./hooks/useConnectionStatus";
import { useNowPlaying } from "./hooks/useNowPlaying";

type Page = "dashboard" | "settings";

function App() {
  const [page, setPage] = useState<Page>("dashboard");

  useConnectionStatus();
  useNowPlaying();

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
