import { useState, useEffect } from "react";
import Dashboard from "./pages/Dashboard";
import Settings from "./pages/Settings";
import { useConnectionStatus } from "./hooks/useConnectionStatus";
import { useNowPlaying } from "./hooks/useNowPlaying";
import { useAppStore } from "./store/appStore";
import i18n from "./i18n";

type Page = "dashboard" | "settings";

function App() {
  const [page, setPage] = useState<Page>("dashboard");
  const language = useAppStore((s) => s.language);

  useConnectionStatus();
  useNowPlaying();

  // language store が変わったら i18next に反映
  useEffect(() => {
    if (language && i18n.language !== language) {
      i18n.changeLanguage(language);
    }
  }, [language]);

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
