/**
 * Main Application
 *
 * Root component for the jamjam P2P audio application.
 * Settings opens as a side panel overlay on the main screen.
 */
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { MainScreen } from "./screens/MainScreen";
import { SettingsPanel } from "./screens/SettingsPanel";
import { SidePanel } from "./components/SidePanel";

function App() {
  const { t } = useTranslation();
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  // Version counter to trigger config reload when settings change
  const [settingsVersion, setSettingsVersion] = useState(0);

  const handleOpenSettings = () => {
    setIsSettingsOpen(true);
  };

  const handleCloseSettings = () => {
    setIsSettingsOpen(false);
    // Increment version to trigger MainScreen config reload
    setSettingsVersion((v) => v + 1);
  };

  return (
    <>
      <MainScreen onSettingsClick={handleOpenSettings} settingsVersion={settingsVersion} />
      <SidePanel
        isOpen={isSettingsOpen}
        onClose={handleCloseSettings}
        title={t("settings.title")}
      >
        <SettingsPanel />
      </SidePanel>
    </>
  );
}

export default App;
