/**
 * Main Screen
 *
 * Entry point for session creation and joining.
 * Displays connection status and provides room management UI.
 */
import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { ConnectionIndicator } from "../components/ConnectionIndicator";
import "./MainScreen.css";

// Session state type
type SessionState =
  | { status: "idle" }
  | { status: "creating" }
  | { status: "joining"; code: string }
  | { status: "connected"; roomCode: string; participants: string[] }
  | { status: "error"; message: string };

export function MainScreen() {
  const { t, i18n } = useTranslation();
  const [sessionState, setSessionState] = useState<SessionState>({ status: "idle" });
  const [inviteCode, setInviteCode] = useState("");

  // Update html lang attribute when language changes
  useEffect(() => {
    document.documentElement.lang = i18n.language;
  }, [i18n.language]);

  // Handle room creation
  const handleCreateRoom = async () => {
    setSessionState({ status: "creating" });

    // TODO: Implement actual room creation via Tauri IPC
    // For now, simulate with timeout
    setTimeout(() => {
      // Simulated success - in real implementation, this would come from backend
      setSessionState({
        status: "connected",
        roomCode: "ABC123",
        participants: [],
      });
    }, 1500);
  };

  // Handle room join
  const handleJoinRoom = async () => {
    if (!inviteCode.trim()) return;

    setSessionState({ status: "joining", code: inviteCode });

    // TODO: Implement actual room joining via Tauri IPC
    // For now, simulate with timeout
    setTimeout(() => {
      if (inviteCode.toUpperCase() === "ERROR") {
        setSessionState({
          status: "error",
          message: t("error.room.notFound.title"),
        });
      } else {
        setSessionState({
          status: "connected",
          roomCode: inviteCode.toUpperCase(),
          participants: ["Host"],
        });
      }
    }, 1500);
  };

  // Handle leave room
  const handleLeaveRoom = () => {
    setSessionState({ status: "idle" });
    setInviteCode("");
  };

  // Handle settings click
  const handleSettingsClick = () => {
    // TODO: Navigate to settings screen
    console.log("Settings clicked");
  };

  // Render idle (disconnected) state
  const renderIdleState = () => (
    <>
      <div className="main-status">
        <ConnectionIndicator status="disconnected" size="lg" />
      </div>

      <div className="main-card">
        <button
          className="main-card__create-btn"
          onClick={handleCreateRoom}
          disabled={sessionState.status !== "idle"}
        >
          {t("session.create.button")}
        </button>

        <div className="main-card__divider">{t("common.label.or", "or")}</div>

        <div className="main-card__join-form">
          <label className="main-card__join-label">{t("session.invite.code")}</label>
          <div className="main-card__join-input-group">
            <input
              type="text"
              className="main-card__join-input"
              placeholder={t("session.join.placeholder")}
              value={inviteCode}
              onChange={(e) => setInviteCode(e.target.value.toUpperCase())}
              maxLength={6}
              disabled={sessionState.status !== "idle"}
            />
            <button
              className="main-card__join-btn"
              onClick={handleJoinRoom}
              disabled={!inviteCode.trim() || sessionState.status !== "idle"}
            >
              {t("session.join.button")}
            </button>
          </div>
        </div>
      </div>

      <div className="main-mode">
        {t("settings.preset.title")}: <span className="main-mode__value">{t("preset.balanced.name")}</span>
      </div>
    </>
  );

  // Render connecting state
  const renderConnectingState = () => (
    <>
      <div className="main-status">
        <ConnectionIndicator status="connecting" size="lg" />
      </div>

      <div className="main-card" style={{ textAlign: "center" }}>
        <p style={{ color: "var(--color-text-secondary)" }}>
          {sessionState.status === "creating"
            ? t("session.create.loading")
            : t("session.join.loading")}
        </p>
      </div>
    </>
  );

  // Render connected state
  const renderConnectedState = () => {
    if (sessionState.status !== "connected") return null;

    return (
      <>
        <div className="main-status">
          <ConnectionIndicator status="connected" latencyMs={15} size="lg" />
        </div>

        <div className="main-card">
          <div style={{ textAlign: "center", marginBottom: "var(--space-lg)" }}>
            <p style={{ color: "var(--color-text-secondary)", marginBottom: "var(--space-xs)" }}>
              {t("session.invite.code")}
            </p>
            <p
              style={{
                fontSize: "var(--font-size-h1)",
                fontFamily: "var(--font-family-mono)",
                letterSpacing: "0.1em",
              }}
            >
              {sessionState.roomCode}
            </p>
          </div>

          <div style={{ marginBottom: "var(--space-lg)" }}>
            <p style={{ color: "var(--color-text-secondary)", marginBottom: "var(--space-sm)" }}>
              {t("session.participant.count", { count: sessionState.participants.length + 1 })}
            </p>
            <ul style={{ listStyle: "none", padding: 0, margin: 0 }}>
              <li
                style={{
                  padding: "var(--space-sm)",
                  backgroundColor: "var(--color-bg-tertiary)",
                  borderRadius: "var(--radius-sm)",
                  marginBottom: "var(--space-xs)",
                }}
              >
                {t("mixer.channel.you")}
              </li>
              {sessionState.participants.map((participant, index) => (
                <li
                  key={index}
                  style={{
                    padding: "var(--space-sm)",
                    backgroundColor: "var(--color-bg-tertiary)",
                    borderRadius: "var(--radius-sm)",
                    marginBottom: "var(--space-xs)",
                  }}
                >
                  {participant}
                </li>
              ))}
            </ul>
          </div>

          <button
            className="main-card__join-btn"
            style={{ width: "100%" }}
            onClick={handleLeaveRoom}
          >
            {t("session.leave.button")}
          </button>
        </div>
      </>
    );
  };

  // Render error state
  const renderErrorState = () => {
    if (sessionState.status !== "error") return null;

    return (
      <>
        <div className="main-status">
          <ConnectionIndicator status="error" size="lg" />
        </div>

        <div className="main-error">{sessionState.message}</div>

        <button
          className="main-card__join-btn"
          onClick={() => setSessionState({ status: "idle" })}
        >
          {t("common.button.retry")}
        </button>
      </>
    );
  };

  return (
    <div className="main-screen">
      <header className="main-header">
        <div className="main-header__logo">
          <span className="main-header__logo-text">jamjam</span>
        </div>
        <button
          className="main-header__settings-btn"
          onClick={handleSettingsClick}
          aria-label={t("settings.title")}
        >
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="12" cy="12" r="3" />
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
          </svg>
        </button>
      </header>

      <main className="main-content">
        {sessionState.status === "idle" && renderIdleState()}
        {(sessionState.status === "creating" || sessionState.status === "joining") &&
          renderConnectingState()}
        {sessionState.status === "connected" && renderConnectedState()}
        {sessionState.status === "error" && renderErrorState()}
      </main>
    </div>
  );
}

export default MainScreen;
