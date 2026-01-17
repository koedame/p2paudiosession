/**
 * Main Screen
 *
 * Entry point for session creation and joining.
 * Displays connection status and provides room management UI.
 */
import { useState, useEffect, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
import { ConnectionIndicator } from "../components/ConnectionIndicator";
import { SessionStats } from "../components/SessionStats";
import { ConnectionHistory } from "../components/ConnectionHistory";
import { MixerConsole, type Participant } from "../components/Mixer";
import { ChatPanel } from "../components/Chat";
import { formatErrorForDisplay } from "../lib/errorMessages";
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
  streamingSetMute,
  audioGetCurrentDevices,
  audioGetBufferSize,
  streamingSetPeerVolume,
  streamingSetMasterVolume,
  streamingSetPeerPan,
  configGetConnectionHistory,
  configAddConnectionHistory,
  configRemoveConnectionHistory,
  configGetPeerName,
  type RoomInfo,
  type PeerInfo,
  type NetworkStats,
  type DetailedLatency,
  type ConnectionHistoryEntry,
} from "../lib/tauri";

// Get signaling server URL from environment variable
const SIGNALING_SERVER_URL = import.meta.env.VITE_SIGNALING_SERVER || "ws://localhost:8080";
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
  /** Version counter to trigger config reload when settings change */
  settingsVersion?: number;
}

export function MainScreen({ onSettingsClick, settingsVersion }: MainScreenProps) {
  const { t, i18n } = useTranslation();
  const [sessionState, setSessionState] = useState<SessionState>({ status: "connecting_server" });
  const [connectionId, setConnectionId] = useState<number | null>(null);
  const [peerName, setPeerName] = useState("User");
  const hasAutoConnected = useRef(false);
  const [roomName, setRoomName] = useState("");
  const [inviteCode, setInviteCode] = useState("");
  const [currentInviteCode, setCurrentInviteCode] = useState("");
  const [networkStats, setNetworkStats] = useState<NetworkStats | null>(null);
  const [detailedLatency, setDetailedLatency] = useState<DetailedLatency | null>(null);
  const [underrunRate, setUnderrunRate] = useState<number>(0);

  // Refs for calculating underrun rate
  const prevUnderrunCount = useRef<number>(0);
  const prevUnderrunTime = useRef<number>(Date.now());
  const [isMuted, setIsMuted] = useState(false);
  const [inputLevel, setInputLevel] = useState(0);
  const [outputLevel, setOutputLevel] = useState(0);
  const [connectionHistory, setConnectionHistory] = useState<ConnectionHistoryEntry[]>([]);
  const [isChatOpen, setIsChatOpen] = useState(false);
  const [myPeerId, setMyPeerId] = useState<string | null>(null);

  // Update html lang attribute when language changes
  useEffect(() => {
    document.documentElement.lang = i18n.language;
  }, [i18n.language]);

  // Load saved configuration and auto-connect on mount
  useEffect(() => {
    const initializeAndConnect = async () => {
      // Load peer name from settings
      try {
        const savedPeerName = await configGetPeerName();
        setPeerName(savedPeerName);
      } catch (e) {
        console.log("Failed to load peer name, using default:", e);
      }

      // Load connection history
      try {
        const history = await configGetConnectionHistory();
        setConnectionHistory(history);
      } catch (e) {
        console.log("Failed to load connection history:", e);
      }

      // Auto-connect to signaling server (only once on initial mount)
      if (!hasAutoConnected.current) {
        hasAutoConnected.current = true;
        await autoConnect();
      }
    };
    initializeAndConnect();
  }, []);

  // Reload peer name when settings change
  useEffect(() => {
    if (settingsVersion === undefined || settingsVersion === 0) return;
    const reloadPeerName = async () => {
      try {
        const savedPeerName = await configGetPeerName();
        setPeerName(savedPeerName);
      } catch (e) {
        console.log("Failed to reload peer name:", e);
      }
    };
    reloadPeerName();
  }, [settingsVersion]);

  // Auto-connect to signaling server
  const autoConnect = async () => {
    setSessionState({ status: "connecting_server" });

    try {
      const connId = await signalingConnect(SIGNALING_SERVER_URL);
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

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (connectionId !== null) {
        signalingDisconnect(connectionId).catch(console.error);
      }
    };
  }, [connectionId]);

  // Poll streaming status for latency and audio level when connected
  useEffect(() => {
    if (sessionState.status !== "connected") {
      setNetworkStats(null);
      setDetailedLatency(null);
      setUnderrunRate(0);
      prevUnderrunCount.current = 0;
      prevUnderrunTime.current = Date.now();
      setInputLevel(0);
      setOutputLevel(0);
      return;
    }

    const pollStats = async () => {
      try {
        const status = await streamingStatus();
        if (status.is_active) {
          setNetworkStats(status.network);
          setDetailedLatency(status.latency);
          setIsMuted(status.is_muted);
          setInputLevel(status.input_level);
          setOutputLevel(status.output_level);

          // Calculate underrun rate (per second)
          if (status.audio_quality) {
            const now = Date.now();
            const currentCount = status.audio_quality.underrun_count;
            const deltaCount = currentCount - prevUnderrunCount.current;
            const deltaTime = (now - prevUnderrunTime.current) / 1000; // seconds

            if (deltaTime > 0) {
              const instantRate = deltaCount / deltaTime;
              // Exponential moving average for smoothing (alpha = 0.3)
              setUnderrunRate((prev) => prev * 0.7 + instantRate * 0.3);
            }

            prevUnderrunCount.current = currentCount;
            prevUnderrunTime.current = now;
          }
        }
      } catch (e) {
        console.error("Failed to get streaming status:", e);
      }
    };

    // Initial poll
    pollStats();

    // Poll every 100ms for smoother audio level updates
    const interval = setInterval(pollStats, 100);

    return () => clearInterval(interval);
  }, [sessionState.status]);

  // Handle reconnect to server
  const handleReconnect = async () => {
    if (connectionId !== null) {
      try {
        await signalingDisconnect(connectionId);
      } catch (e) {
        console.error("Failed to disconnect:", e);
      }
      setConnectionId(null);
    }
    await autoConnect();
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
      setCurrentInviteCode(result.invite_code);
      setMyPeerId(result.peer_id);
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

  // Handle join by invite code
  const handleJoinByCode = async () => {
    if (connectionId === null || !inviteCode.trim()) return;
    await handleJoinRoom(inviteCode.trim().toUpperCase());
  };

  // Handle room join
  const handleJoinRoom = async (roomId: string) => {
    if (connectionId === null) return;

    setSessionState({ status: "joining", code: roomId });

    try {
      const result = await signalingJoinRoom(connectionId, roomId, peerName);
      setCurrentInviteCode(result.invite_code || "");
      setMyPeerId(result.peer_id);
      setSessionState({
        status: "connected",
        roomCode: result.room_id,
        participants: result.peers.map((p) => p.name),
      });

      // Save to connection history
      const historyCode = result.invite_code || roomId.slice(0, 6).toUpperCase();
      try {
        await configAddConnectionHistory(historyCode);
        // Reload history to show updated list
        const history = await configGetConnectionHistory();
        setConnectionHistory(history);
      } catch (historyErr) {
        console.error("Failed to save to history:", historyErr);
      }

      // Start streaming if there's a peer with an address (e.g., echobot)
      const peerWithAddr = result.peers.find(
        (p: PeerInfo) => p.public_addr || p.local_addr
      );
      if (peerWithAddr) {
        const addr = peerWithAddr.public_addr || peerWithAddr.local_addr;
        if (addr) {
          try {
            // Get currently selected devices and buffer size from settings
            const [devices, bufferSize] = await Promise.all([
              audioGetCurrentDevices(),
              audioGetBufferSize(),
            ]);
            await streamingStart(
              addr,
              devices.input_device_id ?? undefined,
              devices.output_device_id ?? undefined,
              bufferSize
            );
            console.log("Streaming started to:", addr, "with devices:", devices, "bufferSize:", bufferSize);
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
      setCurrentInviteCode("");
      setInviteCode("");
      setMyPeerId(null);
      setIsChatOpen(false);
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

  // Handle mute toggle
  const handleToggleMute = useCallback(async () => {
    try {
      const newMuteState = !isMuted;
      await streamingSetMute(newMuteState);
      setIsMuted(newMuteState);
    } catch (e) {
      console.error("Failed to toggle mute:", e);
    }
  }, [isMuted]);

  // Handle selecting from connection history
  const handleHistorySelect = useCallback((roomCode: string) => {
    setInviteCode(roomCode);
  }, []);

  // Handle removing from connection history
  const handleHistoryRemove = useCallback(async (roomCode: string) => {
    try {
      await configRemoveConnectionHistory(roomCode);
      setConnectionHistory((prev) => prev.filter((e) => e.room_code !== roomCode));
    } catch (e) {
      console.error("Failed to remove from history:", e);
    }
  }, []);

  // Handle peer volume change from mixer
  const handlePeerVolumeChange = useCallback(async (_participantId: string, volume: number) => {
    try {
      // Convert 0-100 fader range to 0-200 backend range (100 = unity)
      const backendVolume = Math.round(volume * 2);
      await streamingSetPeerVolume(backendVolume);
    } catch (e) {
      console.error("Failed to set peer volume:", e);
    }
  }, []);

  // Handle master volume change from mixer
  const handleMasterVolumeChange = useCallback(async (volume: number) => {
    try {
      // Convert 0-100 fader range to 0-200 backend range (100 = unity)
      const backendVolume = Math.round(volume * 2);
      await streamingSetMasterVolume(backendVolume);
    } catch (e) {
      console.error("Failed to set master volume:", e);
    }
  }, []);

  // Handle peer pan change from mixer
  const handlePeerPanChange = useCallback(async (_participantId: string, pan: number) => {
    try {
      // Pan is already -100 to 100, send directly to backend
      await streamingSetPeerPan(pan);
    } catch (e) {
      console.error("Failed to set peer pan:", e);
    }
  }, []);

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

          <div className="main-card__join-code-section">
            <label className="main-card__join-label">
              {t("session.invite.joinByCode", "Join by Invite Code")}
            </label>
            <div className="main-card__join-input-group">
              <input
                type="text"
                className="main-card__join-input main-card__join-input--code"
                placeholder="ABC123"
                value={inviteCode}
                onChange={(e) => setInviteCode(e.target.value.toUpperCase().slice(0, 6))}
                maxLength={6}
              />
              <button
                className="main-card__join-btn"
                onClick={handleJoinByCode}
                disabled={inviteCode.length !== 6}
              >
                {t("session.join.button")}
              </button>
            </div>

            {/* Connection history */}
            <ConnectionHistory
              history={connectionHistory}
              onSelect={handleHistorySelect}
              onRemove={handleHistoryRemove}
            />
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
          <ConnectionIndicator
            status="connected"
            upstreamLatencyMs={detailedLatency?.upstream_total_ms}
            downstreamLatencyMs={detailedLatency?.downstream_total_ms}
            size="lg"
          />
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
              {currentInviteCode || sessionState.roomCode.slice(0, 6).toUpperCase()}
            </p>
            <p style={{ color: "var(--color-text-tertiary)", fontSize: "var(--font-size-sm)", marginTop: "var(--space-xs)" }}>
              {t("session.invite.shareHint", "Share this code with others to join")}
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

        {/* Mixer Console - outside card for horizontal expansion */}
        <div className="main-mixer">
          <MixerConsole
            inputLevel={inputLevel}
            isInputMuted={isMuted}
            participants={sessionState.participants.map((name, index): Participant => ({
              id: `peer-${index}`,
              name: name,
              level: outputLevel,
            }))}
            masterLevel={outputLevel}
            onInputMuteToggle={handleToggleMute}
            onParticipantVolumeChange={handlePeerVolumeChange}
            onParticipantPanChange={handlePeerPanChange}
            onMasterVolumeChange={handleMasterVolumeChange}
          />
        </div>

        {/* Detailed Session Statistics */}
        <SessionStats network={networkStats} latency={detailedLatency} underrunRate={underrunRate} />
      </>
    );
  };

  // Render error state
  const renderErrorState = () => {
    if (sessionState.status !== "error") return null;

    const formattedError = formatErrorForDisplay(sessionState.message, t);

    return (
      <>
        <div className="main-status">
          <ConnectionIndicator status="error" size="lg" />
        </div>

        <div className="main-error">
          <div className="main-error__title">{formattedError.title}</div>
          <div className="main-error__message">{formattedError.message}</div>
        </div>

        <button
          className="main-card__join-btn"
          onClick={handleReconnect}
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
        <div className="main-header__actions">
          {sessionState.status === "connected" && (
            <button
              className="main-header__chat-btn"
              onClick={() => setIsChatOpen(true)}
              aria-label={t("chat.title", "Chat")}
            >
              <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
              </svg>
            </button>
          )}
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
        </div>
      </header>

      <main className="main-content">
        {sessionState.status === "server_connected" && renderServerConnectedState()}
        {isConnecting && renderConnectingState()}
        {sessionState.status === "connected" && renderConnectedState()}
        {sessionState.status === "error" && renderErrorState()}
      </main>

      {/* Chat Panel */}
      <ChatPanel
        isOpen={isChatOpen}
        onClose={() => setIsChatOpen(false)}
        connId={connectionId}
        myPeerId={myPeerId}
      />
    </div>
  );
}

export default MainScreen;
