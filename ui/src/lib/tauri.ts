/**
 * Tauri IPC wrapper functions
 *
 * Provides typed interfaces for communicating with the Rust backend.
 */

import { invoke } from "@tauri-apps/api/core";

/**
 * Room information from signaling server
 */
export interface RoomInfo {
  id: string;
  name: string;
  peer_count: number;
  max_peers: number;
  has_password: boolean;
  invite_code: string;
}

/**
 * Peer information
 */
export interface PeerInfo {
  id: string;
  name: string;
  public_addr: string | null;
  local_addr: string | null;
}

/**
 * Result of joining or creating a room
 */
export interface JoinResult {
  room_id: string;
  peer_id: string;
  invite_code: string;
  peers: PeerInfo[];
}

/**
 * Connect to a signaling server
 * @param url WebSocket URL of the signaling server (e.g., ws://localhost:8080)
 * @returns Connection ID for subsequent operations
 */
export async function signalingConnect(url: string): Promise<number> {
  return invoke("signaling_connect", { url });
}

/**
 * Disconnect from a signaling server
 * @param connId Connection ID from signalingConnect
 */
export async function signalingDisconnect(connId: number): Promise<void> {
  return invoke("signaling_disconnect", { connId });
}

/**
 * List available rooms on the signaling server
 * @param connId Connection ID from signalingConnect
 * @returns Array of room information
 */
export async function signalingListRooms(connId: number): Promise<RoomInfo[]> {
  return invoke("signaling_list_rooms", { connId });
}

/**
 * Join an existing room
 * @param connId Connection ID from signalingConnect
 * @param roomId Room ID to join
 * @param peerName Display name for this peer
 * @returns Join result with room info and peer list
 */
export async function signalingJoinRoom(
  connId: number,
  roomId: string,
  peerName: string
): Promise<JoinResult> {
  return invoke("signaling_join_room", { connId, roomId, peerName });
}

/**
 * Leave the current room
 * @param connId Connection ID from signalingConnect
 */
export async function signalingLeaveRoom(connId: number): Promise<void> {
  return invoke("signaling_leave_room", { connId });
}

/**
 * Create a new room
 * @param connId Connection ID from signalingConnect
 * @param roomName Name for the new room
 * @param peerName Display name for this peer (room creator)
 * @returns Join result with the new room info
 */
export async function signalingCreateRoom(
  connId: number,
  roomName: string,
  peerName: string
): Promise<JoinResult> {
  return invoke("signaling_create_room", { connId, roomName, peerName });
}

// ============================================================================
// Audio Device API
// ============================================================================

/**
 * Audio device information
 */
export interface AudioDeviceInfo {
  id: string;
  name: string;
  supported_sample_rates: number[];
  supported_channels: number[];
  is_default: boolean;
  is_asio: boolean;
}

/**
 * Current device selection
 */
export interface CurrentDevices {
  input_device_id: string | null;
  output_device_id: string | null;
}

/**
 * List available input (microphone) devices
 * @returns Array of input device information
 */
export async function audioListInputDevices(): Promise<AudioDeviceInfo[]> {
  return invoke("audio_list_input_devices");
}

/**
 * List available output (speaker) devices
 * @returns Array of output device information
 */
export async function audioListOutputDevices(): Promise<AudioDeviceInfo[]> {
  return invoke("audio_list_output_devices");
}

/**
 * Set the input device
 * @param deviceId Device ID to use, or null for default device
 */
export async function audioSetInputDevice(
  deviceId: string | null
): Promise<void> {
  return invoke("audio_set_input_device", { deviceId });
}

/**
 * Set the output device
 * @param deviceId Device ID to use, or null for default device
 */
export async function audioSetOutputDevice(
  deviceId: string | null
): Promise<void> {
  return invoke("audio_set_output_device", { deviceId });
}

/**
 * Get current device selection
 * @returns Current input and output device IDs
 */
export async function audioGetCurrentDevices(): Promise<CurrentDevices> {
  return invoke("audio_get_current_devices");
}

/**
 * Get current buffer size (frame_size in samples)
 * @returns Buffer size (32, 64, 128, or 256)
 */
export async function audioGetBufferSize(): Promise<number> {
  return invoke("audio_get_buffer_size");
}

/**
 * Set buffer size (frame_size in samples)
 * Lower values = less latency but may cause audio crackling
 * Higher values = more stable but higher latency
 * @param size Buffer size (32, 64, 128, or 256)
 */
export async function audioSetBufferSize(size: number): Promise<void> {
  return invoke("audio_set_buffer_size", { size });
}

// ============================================================================
// Streaming API
// ============================================================================

/**
 * Network statistics
 */
export interface NetworkStats {
  /** Round-trip time in milliseconds */
  rtt_ms: number;
  /** Jitter in milliseconds */
  jitter_ms: number;
  /** Packet loss percentage (0-100) */
  packet_loss_percent: number;
  /** Connection uptime in seconds */
  uptime_seconds: number;
  /** Total packets sent */
  packets_sent: number;
  /** Total packets received */
  packets_received: number;
  /** Total bytes sent */
  bytes_sent: number;
  /** Total bytes received */
  bytes_received: number;
}

/**
 * Latency component breakdown
 */
export interface LatencyComponent {
  /** Component name */
  name: string;
  /** Latency in milliseconds */
  ms: number;
  /** Additional info (e.g., "128 samples @ 48000 Hz") */
  info: string | null;
}

/**
 * Detailed latency breakdown
 */
export interface DetailedLatency {
  /** Upstream components (self -> peer) */
  upstream: LatencyComponent[];
  /** Upstream total in ms */
  upstream_total_ms: number;
  /** Downstream components (peer -> self) */
  downstream: LatencyComponent[];
  /** Downstream total in ms */
  downstream_total_ms: number;
  /** Round-trip total in ms */
  roundtrip_total_ms: number;
}

/**
 * Audio quality metrics
 */
export interface AudioQuality {
  /** Number of buffer underruns (audio glitches due to CPU/scheduling) */
  underrun_count: number;
}

/**
 * Streaming status information
 */
export interface StreamingStatus {
  is_active: boolean;
  remote_addr: string | null;
  /** Whether microphone is muted */
  is_muted: boolean;
  /** Current input audio level (0-100) */
  input_level: number;
  /** Current output audio level (0-100, for master meter) */
  output_level: number;
  /** Network statistics */
  network: NetworkStats | null;
  /** Detailed latency breakdown */
  latency: DetailedLatency | null;
  /** Audio quality metrics */
  audio_quality: AudioQuality | null;
}

/**
 * Start audio streaming to a remote peer
 * @param remoteAddr Remote address in format "ip:port"
 * @param inputDeviceId Optional input device ID
 * @param outputDeviceId Optional output device ID
 * @param bufferSize Buffer size in samples (32, 64, 128, or 256). Default: 64
 */
export async function streamingStart(
  remoteAddr: string,
  inputDeviceId?: string,
  outputDeviceId?: string,
  bufferSize?: number
): Promise<void> {
  return invoke("streaming_start", {
    remoteAddr,
    inputDeviceId: inputDeviceId ?? null,
    outputDeviceId: outputDeviceId ?? null,
    bufferSize: bufferSize ?? 64,
  });
}

/**
 * Stop audio streaming
 */
export async function streamingStop(): Promise<void> {
  return invoke("streaming_stop");
}

/**
 * Get streaming status
 * @returns Current streaming status
 */
export async function streamingStatus(): Promise<StreamingStatus> {
  return invoke("streaming_status");
}

/**
 * Set input device during streaming
 * @param deviceId Device ID to use, or null for default device
 */
export async function streamingSetInputDevice(
  deviceId: string | null
): Promise<void> {
  return invoke("streaming_set_input_device", { deviceId });
}

/**
 * Set output device during streaming
 * @param deviceId Device ID to use, or null for default device
 */
export async function streamingSetOutputDevice(
  deviceId: string | null
): Promise<void> {
  return invoke("streaming_set_output_device", { deviceId });
}

/**
 * Set mute state
 * @param muted Whether to mute the microphone
 */
export async function streamingSetMute(muted: boolean): Promise<void> {
  return invoke("streaming_set_mute", { muted });
}

/**
 * Get mute state
 * @returns Whether the microphone is muted
 */
export async function streamingGetMute(): Promise<boolean> {
  return invoke("streaming_get_mute");
}

/**
 * Get current input audio level
 * @returns Audio level from 0 to 100
 */
export async function streamingGetInputLevel(): Promise<number> {
  return invoke("streaming_get_input_level");
}

/**
 * Set peer (received audio) volume
 * @param volume Volume from 0 to 200 (100 = unity gain, 200 = 2x)
 */
export async function streamingSetPeerVolume(volume: number): Promise<void> {
  return invoke("streaming_set_peer_volume", { volume: Math.round(volume) });
}

/**
 * Get peer volume
 * @returns Volume from 0 to 200 (100 = unity)
 */
export async function streamingGetPeerVolume(): Promise<number> {
  return invoke("streaming_get_peer_volume");
}

/**
 * Set master output volume
 * @param volume Volume from 0 to 200 (100 = unity gain, 200 = 2x)
 */
export async function streamingSetMasterVolume(volume: number): Promise<void> {
  return invoke("streaming_set_master_volume", { volume: Math.round(volume) });
}

/**
 * Get master volume
 * @returns Volume from 0 to 200 (100 = unity)
 */
export async function streamingGetMasterVolume(): Promise<number> {
  return invoke("streaming_get_master_volume");
}

/**
 * Set peer (received audio) pan
 * @param pan Pan from -100 (full left) to 100 (full right), 0 = center
 */
export async function streamingSetPeerPan(pan: number): Promise<void> {
  return invoke("streaming_set_peer_pan", { pan: Math.round(pan) });
}

/**
 * Get peer pan
 * @returns Pan from -100 to 100 (0 = center)
 */
export async function streamingGetPeerPan(): Promise<number> {
  return invoke("streaming_get_peer_pan");
}

// ============================================================================
// Configuration API
// ============================================================================

/**
 * Connection history entry
 */
export interface ConnectionHistoryEntry {
  /** Room code used for the connection */
  room_code: string;
  /** Timestamp of the connection (ISO 8601 format) */
  connected_at: string;
  /** Optional user-defined label for this connection */
  label: string | null;
}

/**
 * Application configuration
 */
export interface AppConfig {
  /** Selected input device ID (null = system default) */
  input_device_id: string | null;
  /** Selected output device ID (null = system default) */
  output_device_id: string | null;
  /** Audio buffer size in samples. Valid values: 32, 64, 128, 256 */
  buffer_size: number;
  /** Custom signaling server URL (null = use default server) */
  signaling_server_url: string | null;
  /** Selected audio preset */
  preset: AudioPresetId;
  /** Connection history (most recent first) */
  connection_history: ConnectionHistoryEntry[];
}

/**
 * Audio preset identifier
 */
export type AudioPresetId =
  | "zero-latency"
  | "ultra-low-latency"
  | "balanced"
  | "high-quality";

/**
 * Preset information
 */
export interface PresetInfo {
  /** Preset identifier (e.g., "zero-latency") */
  id: AudioPresetId;
  /** Recommended buffer size in samples */
  buffer_size: number;
  /** Recommended jitter buffer frames */
  jitter_buffer_frames: number;
}

/**
 * Load configuration from disk
 * @returns The current configuration (from file or defaults if file doesn't exist)
 */
export async function configLoad(): Promise<AppConfig> {
  return invoke("config_load");
}

/**
 * Save configuration to disk
 * @param config The configuration to save
 */
export async function configSave(config: AppConfig): Promise<void> {
  return invoke("config_save", { config });
}

/**
 * Get the signaling server URL from configuration
 * @returns The custom server URL, or null if using default
 */
export async function configGetServerUrl(): Promise<string | null> {
  return invoke("config_get_server_url");
}

/**
 * Set the signaling server URL in configuration
 * @param url The server URL, or null to use default
 */
export async function configSetServerUrl(url: string | null): Promise<void> {
  return invoke("config_set_server_url", { url });
}

// ============================================================================
// Preset API
// ============================================================================

/**
 * List all available presets
 * @returns Array of preset information
 */
export async function configListPresets(): Promise<PresetInfo[]> {
  return invoke("config_list_presets");
}

/**
 * Get the current preset
 * @returns Current preset identifier
 */
export async function configGetPreset(): Promise<AudioPresetId> {
  return invoke("config_get_preset");
}

/**
 * Set the preset and apply its recommended settings
 * @param presetName Preset identifier (e.g., "zero-latency")
 * @returns Applied preset information
 */
export async function configSetPreset(
  presetName: AudioPresetId
): Promise<PresetInfo> {
  return invoke("config_set_preset", { presetName });
}

// ============================================================================
// Connection History API
// ============================================================================

/**
 * Get connection history
 * @returns List of past connections, most recent first
 */
export async function configGetConnectionHistory(): Promise<
  ConnectionHistoryEntry[]
> {
  return invoke("config_get_connection_history");
}

/**
 * Add a connection to history
 * @param roomCode Room code that was used
 * @param label Optional label for this connection
 */
export async function configAddConnectionHistory(
  roomCode: string,
  label?: string
): Promise<void> {
  return invoke("config_add_connection_history", {
    roomCode,
    label: label ?? null,
  });
}

/**
 * Remove a connection from history
 * @param roomCode Room code to remove
 */
export async function configRemoveConnectionHistory(
  roomCode: string
): Promise<void> {
  return invoke("config_remove_connection_history", { roomCode });
}

/**
 * Clear all connection history
 */
export async function configClearConnectionHistory(): Promise<void> {
  return invoke("config_clear_connection_history");
}

/**
 * Update connection history entry label
 * @param roomCode Room code to update
 * @param label New label (or null to remove label)
 */
export async function configUpdateConnectionHistoryLabel(
  roomCode: string,
  label: string | null
): Promise<void> {
  return invoke("config_update_connection_history_label", { roomCode, label });
}
