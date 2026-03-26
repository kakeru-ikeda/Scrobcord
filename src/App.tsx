import { useState, useEffect } from "react";
import Dashboard from "./pages/Dashboard";
import Settings from "./pages/Settings";
import { useConnectionStatus } from "./hooks/useConnectionStatus";
import { useNowPlaying } from "./hooks/useNowPlaying";
import { useAppStore } from "./store/appStore";
import { getSettings } from "./lib/tauriInvoke";
import i18n from "./i18n";

type Page = "dashboard" | "settings";

function App() {
  const [page, setPage] = useState<Page>("dashboard");
  const language = useAppStore((s) => s.language);
  const setLanguage = useAppStore((s) => s.setLanguage);

  useConnectionStatus();
  useNowPlaying();

  // 起動時に保存済み設定から言語を読み込む
  useEffect(() => {
    getSettings()
      .then((s) => {
        if (s.language) setLanguage(s.language);
      })
      .catch(() => {});
  }, []);

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
