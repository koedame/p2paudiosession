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

// ============================================================================
// Streaming API
// ============================================================================

/**
 * Streaming status information
 */
export interface StreamingStatus {
  is_active: boolean;
  remote_addr: string | null;
  /** Round-trip time in milliseconds (measured) */
  rtt_ms: number | null;
  /** Jitter in milliseconds (RTT variation) */
  jitter_ms: number | null;
}

/**
 * Start audio streaming to a remote peer
 * @param remoteAddr Remote address in format "ip:port"
 * @param inputDeviceId Optional input device ID
 * @param outputDeviceId Optional output device ID
 */
export async function streamingStart(
  remoteAddr: string,
  inputDeviceId?: string,
  outputDeviceId?: string
): Promise<void> {
  return invoke("streaming_start", {
    remoteAddr,
    inputDeviceId: inputDeviceId ?? null,
    outputDeviceId: outputDeviceId ?? null,
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
