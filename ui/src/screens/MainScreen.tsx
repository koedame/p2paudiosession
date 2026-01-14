/**
 * Main Screen
 *
 * Entry point for session creation and joining.
 * Displays connection status and provides room management UI.
 */
import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { ConnectionIndicator } from "../components/ConnectionIndicator";
import {
  signalingConnect,
  signalingDisconnect,
  signalingListRooms,
  signalingJoinRoom,
  signalingLeaveRoom,
  signalingCreateRoom,
  streamingStart,
  streamingStop,
  streamingStatus,
  type RoomInfo,
  type PeerInfo,
} from "../lib/tauri";
import "./MainScreen.css";

// Session state type
type SessionState =
  | { status: "idle" }
  | { status: "connecting_server" }
  | { status: "server_connected"; rooms: RoomInfo[] }
  | { status: "creating" }
  | { status: "joining"; code: string }
  | { status: "connected"; roomCode: string; participants: string[] }
  | { status: "error"; message: string };

export interface MainScreenProps {
  onSettingsClick?: () => void;
}

export function MainScreen({ onSettingsClick }: MainScreenProps) {
  const { t, i18n } = useTranslation();
  const [sessionState, setSessionState] = useState<SessionState>({ status: "idle" });
  const [serverUrl, setServerUrl] = useState("ws://localhost:8080");
  const [connectionId, setConnectionId] = useState<number | null>(null);
  const [peerName, setPeerName] = useState("User");
  const [roomName, setRoomName] = useState("");
  const [latencyMs, setLatencyMs] = useState<number | null>(null);

  // Update html lang attribute when language changes
  useEffect(() => {
    document.documentElement.lang = i18n.language;
  }, [i18n.language]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (connectionId !== null) {
        signalingDisconnect(connectionId).catch(console.error);
      }
    };
  }, [connectionId]);

  // Poll streaming status for latency when connected
  useEffect(() => {
    if (sessionState.status !== "connected") {
      setLatencyMs(null);
      return;
    }

    const pollLatency = async () => {
      try {
        const status = await streamingStatus();
        if (status.is_active && status.rtt_ms !== null) {
          setLatencyMs(status.rtt_ms);
        }
      } catch (e) {
        console.error("Failed to get streaming status:", e);
      }
    };

    // Initial poll
    pollLatency();

    // Poll every 500ms
    const interval = setInterval(pollLatency, 500);

    return () => clearInterval(interval);
  }, [sessionState.status]);

  // Handle server connection
  const handleConnectServer = async () => {
    setSessionState({ status: "connecting_server" });

    try {
      const connId = await signalingConnect(serverUrl);
      setConnectionId(connId);

      const rooms = await signalingListRooms(connId);
      setSessionState({ status: "server_connected", rooms });
    } catch (e) {
      setSessionState({
        status: "error",
        message: String(e),
      });
    }
  };

  // Handle disconnect from server
  const handleDisconnectServer = async () => {
    if (connectionId !== null) {
      try {
        await signalingDisconnect(connectionId);
      } catch (e) {
        console.error("Failed to disconnect:", e);
      }
      setConnectionId(null);
    }
    setSessionState({ status: "idle" });
  };

  // Refresh room list
  const handleRefreshRooms = async () => {
    if (connectionId === null) return;

    try {
      const rooms = await signalingListRooms(connectionId);
      setSessionState({ status: "server_connected", rooms });
    } catch (e) {
      setSessionState({
        status: "error",
        message: String(e),
      });
    }
  };

  // Handle room creation
  const handleCreateRoom = async () => {
    if (connectionId === null) return;

    setSessionState({ status: "creating" });

    try {
      const result = await signalingCreateRoom(
        connectionId,
        roomName || "My Room",
        peerName
      );
      setSessionState({
        status: "connected",
        roomCode: result.room_id,
        participants: result.peers.map((p) => p.name),
      });
    } catch (e) {
      setSessionState({
        status: "error",
        message: String(e),
      });
    }
  };

  // Handle room join
  const handleJoinRoom = async (roomId: string) => {
    if (connectionId === null) return;

    setSessionState({ status: "joining", code: roomId });

    try {
      const result = await signalingJoinRoom(connectionId, roomId, peerName);
      setSessionState({
        status: "connected",
        roomCode: result.room_id,
        participants: result.peers.map((p) => p.name),
      });

      // Start streaming if there's a peer with an address (e.g., echobot)
      const peerWithAddr = result.peers.find(
        (p: PeerInfo) => p.public_addr || p.local_addr
      );
      if (peerWithAddr) {
        const addr = peerWithAddr.public_addr || peerWithAddr.local_addr;
        if (addr) {
          try {
            await streamingStart(addr);
            console.log("Streaming started to:", addr);
          } catch (streamErr) {
            console.error("Failed to start streaming:", streamErr);
          }
        }
      }
    } catch (e) {
      setSessionState({
        status: "error",
        message: String(e),
      });
    }
  };

  // Handle leave room
  const handleLeaveRoom = async () => {
    if (connectionId === null) return;

    try {
      // Stop streaming first
      try {
        await streamingStop();
        console.log("Streaming stopped");
      } catch (streamErr) {
        console.error("Failed to stop streaming:", streamErr);
      }

      await signalingLeaveRoom(connectionId);
      const rooms = await signalingListRooms(connectionId);
      setSessionState({ status: "server_connected", rooms });
    } catch (e) {
      setSessionState({
        status: "error",
        message: String(e),
      });
    }
  };

  // Handle settings click
  const handleSettingsClick = () => {
    onSettingsClick?.();
  };

  // Render idle (disconnected from server) state
  const renderIdleState = () => (
    <>
      <div className="main-status">
        <ConnectionIndicator status="disconnected" size="lg" />
      </div>

      <div className="main-card">
        <div className="main-card__server-form">
          <label className="main-card__join-label">
            {t("signaling.server.label", "Signaling Server")}
          </label>
          <div className="main-card__join-input-group">
            <input
              type="text"
              className="main-card__join-input"
              placeholder="ws://localhost:8080"
              value={serverUrl}
              onChange={(e) => setServerUrl(e.target.value)}
            />
            <button
              className="main-card__join-btn"
              onClick={handleConnectServer}
              disabled={!serverUrl.trim()}
            >
              {t("signaling.connect.button", "Connect")}
            </button>
          </div>
        </div>

        <div className="main-card__divider" />

        <div className="main-card__name-form">
          <label className="main-card__join-label">
            {t("signaling.peer.name", "Your Name")}
          </label>
          <input
            type="text"
            className="main-card__join-input"
            placeholder="User"
            value={peerName}
            onChange={(e) => setPeerName(e.target.value)}
          />
        </div>
      </div>

      <div className="main-mode">
        {t("settings.preset.title")}: <span className="main-mode__value">{t("preset.balanced.name")}</span>
      </div>
    </>
  );

  // Render server connected state (room list)
  const renderServerConnectedState = () => {
    if (sessionState.status !== "server_connected") return null;

    return (
      <>
        <div className="main-status">
          <ConnectionIndicator status="connected" size="lg" />
          <span className="main-status__text">
            {t("signaling.connected", "Connected to server")}
          </span>
        </div>

        <div className="main-card">
          <div className="main-card__section">
            <div className="main-card__section-header">
              <h3>{t("signaling.rooms.title", "Available Rooms")}</h3>
              <button
                className="main-card__icon-btn"
                onClick={handleRefreshRooms}
                title={t("common.button.refresh", "Refresh")}
              >
                ↻
              </button>
            </div>

            {sessionState.rooms.length === 0 ? (
              <p className="main-card__empty">
                {t("signaling.rooms.empty", "No rooms available")}
              </p>
            ) : (
              <ul className="main-card__room-list">
                {sessionState.rooms.map((room) => (
                  <li key={room.id} className="main-card__room-item">
                    <div className="main-card__room-info">
                      <span className="main-card__room-name">{room.name}</span>
                      <span className="main-card__room-peers">
                        {room.peer_count}/{room.max_peers}
                      </span>
                    </div>
                    <button
                      className="main-card__join-btn main-card__join-btn--small"
                      onClick={() => handleJoinRoom(room.id)}
                      disabled={room.peer_count >= room.max_peers}
                    >
                      {t("session.join.button")}
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>

          <div className="main-card__divider">{t("common.label.or")}</div>

          <div className="main-card__create-section">
            <label className="main-card__join-label">
              {t("signaling.room.name", "Room Name")}
            </label>
            <div className="main-card__join-input-group">
              <input
                type="text"
                className="main-card__join-input"
                placeholder={t("signaling.room.placeholder", "My Room")}
                value={roomName}
                onChange={(e) => setRoomName(e.target.value)}
              />
              <button
                className="main-card__create-btn"
                onClick={handleCreateRoom}
              >
                {t("session.create.button")}
              </button>
            </div>
          </div>

          <button
            className="main-card__disconnect-btn"
            onClick={handleDisconnectServer}
          >
            {t("signaling.disconnect.button", "Disconnect")}
          </button>
        </div>
      </>
    );
  };

  // Render connecting state
  const renderConnectingState = () => (
    <>
      <div className="main-status">
        <ConnectionIndicator status="connecting" size="lg" />
      </div>

      <div className="main-card" style={{ textAlign: "center" }}>
        <p style={{ color: "var(--color-text-secondary)" }}>
          {sessionState.status === "connecting_server"
            ? t("signaling.connecting", "Connecting to server...")
            : sessionState.status === "creating"
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
          <ConnectionIndicator status="connected" latencyMs={latencyMs ?? undefined} size="lg" />
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
                {peerName} ({t("mixer.channel.you")})
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

  const isConnecting =
    sessionState.status === "connecting_server" ||
    sessionState.status === "creating" ||
    sessionState.status === "joining";

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
        {sessionState.status === "server_connected" && renderServerConnectedState()}
        {isConnecting && renderConnectingState()}
        {sessionState.status === "connected" && renderConnectedState()}
        {sessionState.status === "error" && renderErrorState()}
      </main>
    </div>
  );
}

export default MainScreen;
