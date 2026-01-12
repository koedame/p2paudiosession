// jamjam UI Application
import { invoke } from "@tauri-apps/api/core";
import { initI18n, t, changeLanguage, getCurrentLanguage } from "./i18n";
import type {
  DeviceInfo,
  PeerInfo,
  SessionStatus,
  AudioConfig,
  Screen,
} from "./types";

// Application state
const state = {
  audioRunning: false,
  sessionActive: false,
  localMuted: false,
  peers: [] as PeerInfo[],
  currentScreen: "main" as Screen,
  localAddr: "",
};

// DOM element cache
let elements: Record<string, HTMLElement | null> = {};

// Initialize application
async function init(): Promise<void> {
  console.log("Initializing jamjam UI...");

  // Initialize i18n
  await initI18n();

  // Render initial UI
  renderApp();

  // Cache DOM elements
  cacheElements();

  // Load initial data
  await Promise.all([loadDevices(), loadAudioConfig()]);

  // Setup event listeners
  setupEventListeners();

  // Start status polling
  setInterval(updateStatus, 1000);

  console.log("jamjam UI initialized");
}

// Render the main application structure
function renderApp(): void {
  const app = document.getElementById("app");
  if (!app) return;

  app.innerHTML = `
    <!-- Header -->
    <header class="header">
      <div class="header-left">
        <h1>${t("app.title")}</h1>
        <nav class="nav">
          <button class="nav-btn active" data-screen="main">${t("nav.main")}</button>
          <button class="nav-btn" data-screen="mixer">${t("nav.mixer")}</button>
          <button class="nav-btn" data-screen="settings">${t("nav.settings")}</button>
          <button class="nav-btn" data-screen="connection">${t("nav.connection")}</button>
        </nav>
      </div>
      <div class="status" id="status" role="status" aria-live="polite">
        <span class="status-dot" aria-hidden="true"></span>
        <span class="status-text">${t("status.disconnected")}</span>
      </div>
    </header>

    <!-- Main Content -->
    <main class="main">
      <!-- Main Screen -->
      <div class="screen active" id="screen-main">
        <div class="main-grid">
          ${renderSessionPanel()}
          ${renderAudioPanel()}
          ${renderMixerPanel()}
        </div>
      </div>

      <!-- Mixer Screen -->
      <div class="screen" id="screen-mixer">
        ${renderFullMixerPanel()}
      </div>

      <!-- Settings Screen -->
      <div class="screen" id="screen-settings">
        ${renderSettingsPanel()}
      </div>

      <!-- Connection Screen -->
      <div class="screen" id="screen-connection">
        ${renderConnectionPanel()}
      </div>
    </main>

    <!-- Footer -->
    <footer class="footer">
      <span>${t("app.title")} ${t("app.version")}</span>
      <span id="latency">RTT: --</span>
    </footer>

    <!-- Toast Container -->
    <div class="toast-container" id="toast-container" role="status" aria-live="polite"></div>
  `;
}

function renderSessionPanel(): string {
  return `
    <section class="panel">
      <h2>${t("session.title")}</h2>
      <div class="tabs">
        <button class="tab-btn active" data-tab="create">${t("session.create")}</button>
        <button class="tab-btn" data-tab="join">${t("session.join")}</button>
        <button class="tab-btn" data-tab="direct">${t("session.directConnect")}</button>
      </div>

      <div class="tab-content active" id="tab-create">
        <div class="form-group">
          <label for="port">${t("session.port")}</label>
          <input type="number" id="port" value="5000" min="1024" max="65535">
        </div>
        <div class="button-group">
          <button id="btn-create" class="btn btn-primary">${t("session.create")}</button>
          <button id="btn-leave" class="btn btn-danger" disabled>${t("session.leave")}</button>
        </div>
      </div>

      <div class="tab-content" id="tab-join">
        <div class="form-group">
          <label for="room-code">${t("session.roomCode")}</label>
          <input type="text" id="room-code" placeholder="ABC123" maxlength="8">
        </div>
        <div class="form-group">
          <label for="room-password">${t("session.password")}</label>
          <input type="password" id="room-password">
        </div>
        <div class="button-group">
          <button id="btn-join" class="btn btn-primary">${t("session.join")}</button>
        </div>
      </div>

      <div class="tab-content" id="tab-direct">
        <div class="form-group">
          <label for="direct-address">${t("session.address")}</label>
          <input type="text" id="direct-address" placeholder="192.168.1.100:5000">
        </div>
        <div class="button-group">
          <button id="btn-direct-connect" class="btn btn-primary">${t("session.join")}</button>
        </div>
      </div>

      <div class="session-info" id="session-info" style="display: none;">
        <p>${t("session.localAddress")}: <span id="local-addr">-</span></p>
        <p>${t("session.peers")}: <span id="peer-count">0</span></p>
        <div class="invite-code">
          <input type="text" id="invite-code" readonly>
          <button class="btn btn-secondary" id="btn-copy-invite">${t("session.copyInvite")}</button>
        </div>
      </div>
    </section>
  `;
}

function renderAudioPanel(): string {
  return `
    <section class="panel">
      <h2>${t("audio.title")}</h2>
      <div class="form-group">
        <label for="input-device">${t("audio.inputDevice")}</label>
        <select id="input-device">
          <option value="">${t("audio.default")}</option>
        </select>
      </div>
      <div class="form-group">
        <label for="output-device">${t("audio.outputDevice")}</label>
        <select id="output-device">
          <option value="">${t("audio.default")}</option>
        </select>
      </div>
      <div class="form-row">
        <div class="form-group">
          <label for="sample-rate">${t("audio.sampleRate")}</label>
          <select id="sample-rate">
            <option value="44100">44.1 kHz</option>
            <option value="48000" selected>48 kHz</option>
            <option value="96000">96 kHz</option>
          </select>
        </div>
        <div class="form-group">
          <label for="frame-size">${t("audio.frameSize")}</label>
          <select id="frame-size">
            <option value="64">64 samples</option>
            <option value="128" selected>128 samples</option>
            <option value="256">256 samples</option>
            <option value="512">512 samples</option>
          </select>
        </div>
      </div>
      <div class="checkbox-group">
        <input type="checkbox" id="local-monitor">
        <label for="local-monitor">${t("audio.localMonitoring")}</label>
      </div>
      <div class="button-group">
        <button id="btn-start-audio" class="btn btn-success">${t("audio.start")}</button>
        <button id="btn-stop-audio" class="btn btn-danger" disabled>${t("audio.stop")}</button>
      </div>
    </section>
  `;
}

function renderMixerPanel(): string {
  return `
    <section class="panel" style="grid-column: 1 / -1;">
      <h2>${t("mixer.title")}</h2>
      <div class="mixer-channels" id="mixer-channels">
        ${renderLocalChannel()}
      </div>
    </section>
  `;
}

function renderLocalChannel(): string {
  return `
    <div class="channel local">
      <div class="channel-label">${t("mixer.you")}</div>
      <div class="meter" aria-hidden="true">
        <div class="meter-fill" id="meter-local"></div>
      </div>
      <input type="range" class="volume-slider" id="volume-local" min="0" max="100" value="100"
        aria-label="${t("mixer.volume")}"
        aria-valuemin="0" aria-valuemax="100" aria-valuenow="100">
      <div class="channel-controls">
        <button class="btn-icon" id="mute-local" title="${t("mixer.mute")}"
          aria-label="${t("mixer.mute")}" aria-pressed="false">
          <span class="icon" aria-hidden="true">M</span>
        </button>
      </div>
    </div>
  `;
}

function renderFullMixerPanel(): string {
  return `
    <section class="panel">
      <h2>${t("mixer.title")}</h2>
      <div class="mixer-channels" id="mixer-channels-full">
        ${renderLocalChannel()}
      </div>
    </section>
  `;
}

function renderSettingsPanel(): string {
  return `
    <section class="panel">
      <h2>${t("settings.title")}</h2>

      <div class="settings-section">
        <h3>${t("settings.language")}</h3>
        <div class="form-group">
          <select id="language-select">
            <option value="en" ${getCurrentLanguage() === "en" ? "selected" : ""}>English</option>
            <option value="ja" ${getCurrentLanguage() === "ja" ? "selected" : ""}>日本語</option>
          </select>
        </div>
      </div>

      <div class="settings-section">
        <h3>${t("settings.preset")}</h3>
        <div class="form-group">
          <select id="preset-select">
            <option value="ultra-low-latency">${t("settings.presets.ultraLowLatency")}</option>
            <option value="balanced" selected>${t("settings.presets.balanced")}</option>
            <option value="high-quality">${t("settings.presets.highQuality")}</option>
          </select>
        </div>
      </div>

      <div class="settings-section">
        <h3>${t("settings.advanced")}</h3>
        <div class="form-group">
          <label for="codec-select">${t("settings.codec")}</label>
          <select id="codec-select">
            <option value="pcm">PCM (Uncompressed)</option>
            <option value="opus">Opus</option>
          </select>
        </div>
        <div class="checkbox-group">
          <input type="checkbox" id="encryption-toggle">
          <label for="encryption-toggle">${t("settings.encryption")}</label>
        </div>
        <div class="form-group">
          <label for="jitter-buffer">${t("settings.jitterBuffer")}</label>
          <input type="number" id="jitter-buffer" value="20" min="5" max="200">
        </div>
        <div class="checkbox-group">
          <input type="checkbox" id="fec-toggle" checked>
          <label for="fec-toggle">${t("settings.fec")}</label>
        </div>
      </div>
    </section>
  `;
}

function renderConnectionPanel(): string {
  return `
    <section class="panel">
      <h2>${t("connection.title")}</h2>
      <div class="stats-grid">
        <div class="stat-card">
          <div class="stat-label">${t("connection.rtt")}</div>
          <div class="stat-value" id="stat-rtt">-- ms</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">${t("connection.packetLoss")}</div>
          <div class="stat-value" id="stat-packet-loss">-- %</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">${t("connection.jitter")}</div>
          <div class="stat-value" id="stat-jitter">-- ms</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">${t("connection.bandwidth")}</div>
          <div class="stat-value" id="stat-bandwidth">-- kbps</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">${t("connection.packetsSent")}</div>
          <div class="stat-value" id="stat-packets-sent">0</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">${t("connection.packetsReceived")}</div>
          <div class="stat-value" id="stat-packets-received">0</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">${t("connection.bytesSent")}</div>
          <div class="stat-value" id="stat-bytes-sent">0 B</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">${t("connection.bytesReceived")}</div>
          <div class="stat-value" id="stat-bytes-received">0 B</div>
        </div>
      </div>
    </section>
  `;
}

// Cache frequently accessed DOM elements
function cacheElements(): void {
  elements = {
    status: document.getElementById("status"),
    port: document.getElementById("port"),
    btnCreate: document.getElementById("btn-create"),
    btnLeave: document.getElementById("btn-leave"),
    btnJoin: document.getElementById("btn-join"),
    sessionInfo: document.getElementById("session-info"),
    localAddr: document.getElementById("local-addr"),
    peerCount: document.getElementById("peer-count"),
    inviteCode: document.getElementById("invite-code"),
    inputDevice: document.getElementById("input-device"),
    outputDevice: document.getElementById("output-device"),
    sampleRate: document.getElementById("sample-rate"),
    frameSize: document.getElementById("frame-size"),
    localMonitor: document.getElementById("local-monitor"),
    btnStartAudio: document.getElementById("btn-start-audio"),
    btnStopAudio: document.getElementById("btn-stop-audio"),
    mixerChannels: document.getElementById("mixer-channels"),
    muteLocal: document.getElementById("mute-local"),
    languageSelect: document.getElementById("language-select"),
  };
}

// Load audio devices
async function loadDevices(): Promise<void> {
  try {
    const [inputDevices, outputDevices] = await Promise.all([
      invoke<DeviceInfo[]>("cmd_get_input_devices"),
      invoke<DeviceInfo[]>("cmd_get_output_devices"),
    ]);

    const inputSelect = elements.inputDevice as HTMLSelectElement;
    const outputSelect = elements.outputDevice as HTMLSelectElement;

    if (inputSelect) {
      inputSelect.innerHTML = `<option value="">${t("audio.default")}</option>`;
      inputDevices.forEach((device) => {
        const option = document.createElement("option");
        option.value = device.id;
        option.textContent =
          device.name + (device.is_default ? " (default)" : "");
        inputSelect.appendChild(option);
      });
    }

    if (outputSelect) {
      outputSelect.innerHTML = `<option value="">${t("audio.default")}</option>`;
      outputDevices.forEach((device) => {
        const option = document.createElement("option");
        option.value = device.id;
        option.textContent =
          device.name + (device.is_default ? " (default)" : "");
        outputSelect.appendChild(option);
      });
    }
  } catch (error) {
    console.error("Failed to load devices:", error);
  }
}

// Load audio configuration
async function loadAudioConfig(): Promise<void> {
  try {
    const config = await invoke<AudioConfig>("cmd_get_audio_config");
    const sampleRate = elements.sampleRate as HTMLSelectElement;
    const frameSize = elements.frameSize as HTMLSelectElement;

    if (sampleRate) sampleRate.value = config.sample_rate.toString();
    if (frameSize) frameSize.value = config.frame_size.toString();
  } catch (error) {
    console.error("Failed to load audio config:", error);
  }
}

// Setup event listeners
function setupEventListeners(): void {
  // Navigation
  document.querySelectorAll(".nav-btn").forEach((btn) => {
    btn.addEventListener("click", (e) => {
      const target = e.target as HTMLElement;
      const screen = target.dataset.screen as Screen;
      if (screen) switchScreen(screen);
    });
  });

  // Tabs
  document.querySelectorAll(".tab-btn").forEach((btn) => {
    btn.addEventListener("click", (e) => {
      const target = e.target as HTMLElement;
      const tab = target.dataset.tab;
      if (tab) switchTab(tab);
    });
  });

  // Session controls
  elements.btnCreate?.addEventListener("click", createSession);
  elements.btnLeave?.addEventListener("click", leaveSession);
  elements.btnJoin?.addEventListener("click", joinSession);
  document
    .getElementById("btn-direct-connect")
    ?.addEventListener("click", directConnect);
  document
    .getElementById("btn-copy-invite")
    ?.addEventListener("click", copyInvite);

  // Audio controls
  elements.btnStartAudio?.addEventListener("click", startAudio);
  elements.btnStopAudio?.addEventListener("click", stopAudio);
  (elements.localMonitor as HTMLInputElement)?.addEventListener(
    "change",
    toggleLocalMonitor
  );

  // Audio settings
  elements.sampleRate?.addEventListener("change", updateAudioConfig);
  elements.frameSize?.addEventListener("change", updateAudioConfig);

  // Local mute
  elements.muteLocal?.addEventListener("click", toggleLocalMute);

  // Language change
  elements.languageSelect?.addEventListener("change", async (e) => {
    const select = e.target as HTMLSelectElement;
    await changeLanguage(select.value);
    renderApp();
    cacheElements();
    await loadDevices();
    setupEventListeners();
  });
}

// Switch between screens
function switchScreen(screen: Screen): void {
  state.currentScreen = screen;

  // Update nav buttons
  document.querySelectorAll(".nav-btn").forEach((btn) => {
    btn.classList.toggle("active", btn.getAttribute("data-screen") === screen);
  });

  // Update screens
  document.querySelectorAll(".screen").forEach((s) => {
    s.classList.toggle("active", s.id === `screen-${screen}`);
  });
}

// Switch tabs within a panel
function switchTab(tab: string): void {
  const tabBtns = document.querySelectorAll(".tab-btn");
  const tabContents = document.querySelectorAll(".tab-content");

  tabBtns.forEach((btn) => {
    btn.classList.toggle("active", btn.getAttribute("data-tab") === tab);
  });

  tabContents.forEach((content) => {
    content.classList.toggle("active", content.id === `tab-${tab}`);
  });
}

// Create session
async function createSession(): Promise<void> {
  try {
    const portInput = elements.port as HTMLInputElement;
    const port = parseInt(portInput?.value || "5000");
    const localAddr = await invoke<string>("cmd_create_session", { port });

    state.sessionActive = true;
    state.localAddr = localAddr;
    updateSessionUI(localAddr);
    showToast(t("session.create") + " OK", "success");
  } catch (error) {
    console.error("Failed to create session:", error);
    showToast(t("error.sessionCreateFailed") + ": " + error, "error");
  }
}

// Join session by room code
async function joinSession(): Promise<void> {
  const roomCode = (document.getElementById("room-code") as HTMLInputElement)
    ?.value;
  const _password = (
    document.getElementById("room-password") as HTMLInputElement
  )?.value;

  if (!roomCode) {
    showToast(t("validation.enterRoomCode"), "error");
    return;
  }

  // TODO: Implement signaling server connection
  // Will use _password when signaling is implemented
  showToast(t("info.roomCodeNotImplemented"), "info");
}

// Direct connect to IP:Port
async function directConnect(): Promise<void> {
  const address = (
    document.getElementById("direct-address") as HTMLInputElement
  )?.value;

  if (!address) {
    showToast(t("validation.enterAddress"), "error");
    return;
  }

  // TODO: Implement direct connection
  showToast(t("info.directConnectNotImplemented"), "info");
}

// Leave session
async function leaveSession(): Promise<void> {
  try {
    await invoke("cmd_leave_session");
    state.sessionActive = false;
    state.localAddr = "";
    updateSessionUI(null);
    showToast(t("session.leave") + " OK", "info");
  } catch (error) {
    console.error("Failed to leave session:", error);
  }
}

// Copy invite code
function copyInvite(): void {
  const inviteInput = elements.inviteCode as HTMLInputElement;
  if (inviteInput) {
    navigator.clipboard.writeText(inviteInput.value);
    showToast(t("common.copied"), "success");
  }
}

// Start audio
async function startAudio(): Promise<void> {
  try {
    await updateAudioConfig();

    const inputDevice =
      (elements.inputDevice as HTMLSelectElement)?.value || null;
    const outputDevice =
      (elements.outputDevice as HTMLSelectElement)?.value || null;

    await invoke("cmd_start_audio", { inputDevice, outputDevice });

    state.audioRunning = true;
    (elements.btnStartAudio as HTMLButtonElement).disabled = true;
    (elements.btnStopAudio as HTMLButtonElement).disabled = false;
    showToast(t("audio.start") + " OK", "success");
  } catch (error) {
    console.error("Failed to start audio:", error);
    showToast(t("error.audioStartFailed") + ": " + error, "error");
  }
}

// Stop audio
async function stopAudio(): Promise<void> {
  try {
    await invoke("cmd_stop_audio");

    state.audioRunning = false;
    (elements.btnStartAudio as HTMLButtonElement).disabled = false;
    (elements.btnStopAudio as HTMLButtonElement).disabled = true;
    showToast(t("audio.stop") + " OK", "info");
  } catch (error) {
    console.error("Failed to stop audio:", error);
  }
}

// Toggle local monitoring
async function toggleLocalMonitor(): Promise<void> {
  try {
    const checkbox = elements.localMonitor as HTMLInputElement;
    const enabled = checkbox?.checked || false;
    await invoke("cmd_set_local_monitoring", { enabled });
  } catch (error) {
    console.error("Failed to toggle local monitoring:", error);
    const checkbox = elements.localMonitor as HTMLInputElement;
    if (checkbox) checkbox.checked = !checkbox.checked;
  }
}

// Update audio configuration
async function updateAudioConfig(): Promise<void> {
  try {
    const sampleRate = parseInt(
      (elements.sampleRate as HTMLSelectElement)?.value || "48000"
    );
    const frameSize = parseInt(
      (elements.frameSize as HTMLSelectElement)?.value || "128"
    );

    await invoke("cmd_set_audio_config", {
      sampleRate,
      channels: 1,
      frameSize,
    });
  } catch (error) {
    console.error("Failed to update audio config:", error);
  }
}

// Toggle local mute
function toggleLocalMute(): void {
  state.localMuted = !state.localMuted;
  const muteBtn = elements.muteLocal;
  if (muteBtn) {
    const icon = muteBtn.querySelector(".icon");
    if (icon) icon.textContent = state.localMuted ? "M" : "M";
    muteBtn.classList.toggle("active", state.localMuted);
    muteBtn.setAttribute("aria-pressed", state.localMuted.toString());
    muteBtn.setAttribute(
      "aria-label",
      state.localMuted ? t("mixer.unmute") : t("mixer.mute")
    );
  }
}

// Update session UI
function updateSessionUI(localAddr: string | null): void {
  const sessionInfo = elements.sessionInfo;
  const btnCreate = elements.btnCreate as HTMLButtonElement;
  const btnLeave = elements.btnLeave as HTMLButtonElement;
  const statusEl = elements.status;

  if (localAddr) {
    if (sessionInfo) sessionInfo.style.display = "block";
    if (elements.localAddr) elements.localAddr.textContent = localAddr;

    // Generate invite code from local address
    const inviteCode = elements.inviteCode as HTMLInputElement;
    if (inviteCode) {
      // Simple hash-like code from address
      const code = localAddr.split(":")[1] || "00000";
      inviteCode.value = code.padStart(8, "0").slice(-8);
    }

    if (btnCreate) btnCreate.disabled = true;
    if (btnLeave) btnLeave.disabled = false;

    if (statusEl) {
      const dot = statusEl.querySelector(".status-dot");
      const text = statusEl.querySelector(".status-text");
      dot?.classList.add("connected");
      if (text) text.textContent = t("status.connected");
    }
  } else {
    if (sessionInfo) sessionInfo.style.display = "none";
    if (btnCreate) btnCreate.disabled = false;
    if (btnLeave) btnLeave.disabled = true;

    if (statusEl) {
      const dot = statusEl.querySelector(".status-dot");
      const text = statusEl.querySelector(".status-text");
      dot?.classList.remove("connected");
      if (text) text.textContent = t("status.disconnected");
    }
  }
}

// Update status periodically
async function updateStatus(): Promise<void> {
  try {
    const status = await invoke<SessionStatus>("cmd_get_session_status");
    const peerCount = elements.peerCount;
    const localMonitor = elements.localMonitor as HTMLInputElement;

    if (peerCount) peerCount.textContent = status.peer_count.toString();
    if (localMonitor) localMonitor.checked = status.local_monitoring;

    // Update peers in mixer
    if (status.connected) {
      const peerList = await invoke<PeerInfo[]>("cmd_get_peers");
      updateMixerPeers(peerList);
    }
  } catch {
    // Ignore errors during polling
  }
}

// Update mixer with peers
function updateMixerPeers(peerList: PeerInfo[]): void {
  const mixerChannels = elements.mixerChannels;
  if (!mixerChannels) return;

  // Remove old peer channels (keep local)
  const existingChannels = mixerChannels.querySelectorAll(
    ".channel:not(.local)"
  );
  existingChannels.forEach((ch) => ch.remove());

  // Add peer channels
  peerList.forEach((peer) => {
    const channel = createPeerChannel(peer);
    mixerChannels.appendChild(channel);
  });
}

// Create a peer channel element
function createPeerChannel(peer: PeerInfo): HTMLElement {
  const channel = document.createElement("div");
  channel.className = "channel peer";
  channel.dataset.peerId = peer.id;

  channel.innerHTML = `
    <div class="channel-label">${escapeHtml(peer.name)}</div>
    <div class="meter" aria-hidden="true">
      <div class="meter-fill" id="meter-${peer.id}"></div>
    </div>
    <input type="range" class="volume-slider" id="volume-${peer.id}" min="0" max="100" value="100"
      aria-label="${t("mixer.volume")} - ${escapeHtml(peer.name)}"
      aria-valuemin="0" aria-valuemax="100" aria-valuenow="100">
    <div class="channel-controls">
      <button class="btn-icon" title="${t("mixer.mute")}"
        aria-label="${t("mixer.mute")} - ${escapeHtml(peer.name)}" aria-pressed="false">
        <span class="icon" aria-hidden="true">M</span>
      </button>
    </div>
  `;

  // Add mute handler
  const muteBtn = channel.querySelector(".btn-icon");
  let muted = false;
  muteBtn?.addEventListener("click", () => {
    muted = !muted;
    muteBtn.classList.toggle("active", muted);
  });

  return channel;
}

// Show toast notification
function showToast(
  message: string,
  type: "success" | "error" | "info" | "warning" = "info"
): void {
  const container = document.getElementById("toast-container");
  if (!container) return;

  const toast = document.createElement("div");
  toast.className = `toast ${type}`;

  // Duration based on type (per UI/UX guideline)
  const durations: Record<string, number> = {
    success: 3000,
    info: 4000,
    warning: 5000,
    error: 0, // Infinite - requires manual close
  };

  const duration = durations[type] || 4000;
  const needsCloseButton = type === "warning" || type === "error";

  if (needsCloseButton) {
    toast.innerHTML = `
      <span class="toast-message">${escapeHtml(message)}</span>
      <button class="toast-close" aria-label="${t("common.close")}">×</button>
    `;
    const closeBtn = toast.querySelector(".toast-close");
    closeBtn?.addEventListener("click", () => toast.remove());
  } else {
    toast.textContent = message;
  }

  container.appendChild(toast);

  // Auto remove after duration (if not infinite)
  if (duration > 0) {
    setTimeout(() => {
      toast.remove();
    }, duration);
  }
}

// Escape HTML
function escapeHtml(text: string): string {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}

// Initialize when DOM is ready
document.addEventListener("DOMContentLoaded", init);
