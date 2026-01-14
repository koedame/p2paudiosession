/**
 * Main Application
 *
 * Root component for the jamjam P2P audio application.
 */
import { useState } from "react";
import { MainScreen } from "./screens/MainScreen";
import { SettingsScreen } from "./screens/SettingsScreen";

type Screen = "main" | "settings";

function App() {
  const [currentScreen, setCurrentScreen] = useState<Screen>("main");

  const handleOpenSettings = () => {
    setCurrentScreen("settings");
  };

  const handleCloseSettings = () => {
    setCurrentScreen("main");
  };

  if (currentScreen === "settings") {
    return <SettingsScreen onBack={handleCloseSettings} />;
  }

  return <MainScreen onSettingsClick={handleOpenSettings} />;
}

export default App;
