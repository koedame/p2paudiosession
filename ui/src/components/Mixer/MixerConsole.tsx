/**
 * MixerConsole - DAW-style mixing console with multiple channel strips
 *
 * Manages audio levels, volumes, and routing for all participants.
 */

import { useState, useCallback, useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { MixerChannel } from "./MixerChannel";
import "./MixerConsole.css";

export interface Participant {
  id: string;
  name: string;
  level: number;
  color?: string;
}

export interface MixerConsoleProps {
  /** Local user's input level (0-100) */
  inputLevel: number;
  /** Whether local input is muted */
  isInputMuted: boolean;
  /** List of remote participants */
  participants: Participant[];
  /** Master output level (0-100) */
  masterLevel?: number;
  /** Callback when local mute is toggled */
  onInputMuteToggle?: () => void;
  /** Callback when participant volume changes */
  onParticipantVolumeChange?: (participantId: string, volume: number) => void;
  /** Callback when participant mute is toggled */
  onParticipantMuteToggle?: (participantId: string) => void;
  /** Callback when participant pan changes */
  onParticipantPanChange?: (participantId: string, pan: number) => void;
  /** Callback when master volume changes */
  onMasterVolumeChange?: (volume: number) => void;
  /** Callback when master pan changes */
  onMasterPanChange?: (pan: number) => void;
}

// Default colors for participants (cycling)
const PARTICIPANT_COLORS = [
  "#4ecdc4", // teal
  "#ff6b6b", // coral
  "#95e1d3", // mint
  "#f38181", // salmon
  "#aa96da", // lavender
  "#fcbad3", // pink
  "#a8d8ea", // sky blue
  "#ffcfdf", // rose
];

interface ChannelState {
  volume: number;
  pan: number;
  isMuted: boolean;
  isSoloed: boolean;
}

export function MixerConsole({
  inputLevel,
  isInputMuted,
  participants,
  masterLevel = 0,
  onInputMuteToggle,
  onParticipantVolumeChange,
  onParticipantMuteToggle,
  onParticipantPanChange,
  onMasterVolumeChange,
  onMasterPanChange,
}: MixerConsoleProps) {
  const { t } = useTranslation();

  // Channel states (volume, pan, mute, solo for each channel)
  const [inputState, setInputState] = useState<ChannelState>({
    volume: 80,
    pan: 0,
    isMuted: isInputMuted,
    isSoloed: false,
  });

  const [participantStates, setParticipantStates] = useState<Map<string, ChannelState>>(new Map());

  const [masterState, setMasterState] = useState<ChannelState>({
    volume: 80,
    pan: 0,
    isMuted: false,
    isSoloed: false,
  });

  // Sync input mute state with prop
  useEffect(() => {
    setInputState((prev) => ({ ...prev, isMuted: isInputMuted }));
  }, [isInputMuted]);

  // Initialize states for new participants
  useEffect(() => {
    setParticipantStates((prevStates) => {
      const newStates = new Map(prevStates);
      participants.forEach((p) => {
        if (!newStates.has(p.id)) {
          newStates.set(p.id, {
            volume: 80,
            pan: 0,
            isMuted: false,
            isSoloed: false,
          });
        }
      });
      return newStates;
    });
  }, [participants]);

  // Check if any channel is soloed (enables solo mode)
  const hasAnySolo = useMemo(() => {
    if (inputState.isSoloed) return true;
    for (const state of participantStates.values()) {
      if (state.isSoloed) return true;
    }
    return false;
  }, [inputState.isSoloed, participantStates]);

  // Apply solo mode to backend volumes
  // When solo is active on some channels, non-soloed channels should be muted
  useEffect(() => {
    participants.forEach((participant) => {
      const state = participantStates.get(participant.id);
      if (state) {
        const shouldMute = state.isMuted || (hasAnySolo && !state.isSoloed);
        // Send effective volume to backend (0 if muted/solo-muted, otherwise the stored volume)
        const effectiveVolume = shouldMute ? 0 : state.volume;
        onParticipantVolumeChange?.(participant.id, effectiveVolume);
      }
    });
  }, [hasAnySolo, participantStates, participants, onParticipantVolumeChange]);

  // Handle input volume change
  const handleInputVolumeChange = useCallback((_id: string, volume: number) => {
    setInputState((prev) => ({ ...prev, volume }));
  }, []);

  // Handle input pan change
  const handleInputPanChange = useCallback((_id: string, pan: number) => {
    setInputState((prev) => ({ ...prev, pan }));
  }, []);

  // Handle input mute toggle
  const handleInputMuteToggle = useCallback(() => {
    onInputMuteToggle?.();
  }, [onInputMuteToggle]);

  // Handle input solo toggle
  const handleInputSoloToggle = useCallback(() => {
    setInputState((prev) => ({ ...prev, isSoloed: !prev.isSoloed }));
  }, []);

  // Handle participant volume change
  const handleParticipantVolumeChange = useCallback(
    (id: string, volume: number) => {
      setParticipantStates((prev) => {
        const newStates = new Map(prev);
        const current = newStates.get(id);
        if (current) {
          newStates.set(id, { ...current, volume });
        }
        return newStates;
      });
      onParticipantVolumeChange?.(id, volume);
    },
    [onParticipantVolumeChange]
  );

  // Handle participant pan change
  const handleParticipantPanChange = useCallback((id: string, pan: number) => {
    setParticipantStates((prev) => {
      const newStates = new Map(prev);
      const current = newStates.get(id);
      if (current) {
        newStates.set(id, { ...current, pan });
      }
      return newStates;
    });
    onParticipantPanChange?.(id, pan);
  }, [onParticipantPanChange]);

  // Handle participant mute toggle
  const handleParticipantMuteToggle = useCallback(
    (id: string) => {
      setParticipantStates((prev) => {
        const newStates = new Map(prev);
        const current = newStates.get(id);
        if (current) {
          newStates.set(id, { ...current, isMuted: !current.isMuted });
        }
        return newStates;
      });
      onParticipantMuteToggle?.(id);
    },
    [onParticipantMuteToggle]
  );

  // Handle participant solo toggle
  const handleParticipantSoloToggle = useCallback((id: string) => {
    setParticipantStates((prev) => {
      const newStates = new Map(prev);
      const current = newStates.get(id);
      if (current) {
        newStates.set(id, { ...current, isSoloed: !current.isSoloed });
      }
      return newStates;
    });
  }, []);

  // Handle master volume change
  const handleMasterVolumeChange = useCallback(
    (_id: string, volume: number) => {
      setMasterState((prev) => ({ ...prev, volume }));
      onMasterVolumeChange?.(volume);
    },
    [onMasterVolumeChange]
  );

  // Handle master pan change
  const handleMasterPanChange = useCallback((_id: string, pan: number) => {
    setMasterState((prev) => ({ ...prev, pan }));
    onMasterPanChange?.(pan);
  }, [onMasterPanChange]);

  // Handle master mute toggle
  const handleMasterMuteToggle = useCallback(() => {
    setMasterState((prev) => ({ ...prev, isMuted: !prev.isMuted }));
  }, []);

  return (
    <div className="mixer-console">
      <div className="mixer-console__channels">
        {/* Input (Self) Channel */}
        <MixerChannel
          id="input"
          name={t("mixer.self", "You")}
          type="input"
          level={inputLevel}
          volume={inputState.volume}
          pan={inputState.pan}
          isMuted={inputState.isMuted || (hasAnySolo && !inputState.isSoloed)}
          isSoloed={inputState.isSoloed}
          color="#ff6b35"
          onVolumeChange={handleInputVolumeChange}
          onPanChange={handleInputPanChange}
          onMuteToggle={handleInputMuteToggle}
          onSoloToggle={handleInputSoloToggle}
        />

        {/* Participant Channels */}
        {participants.map((participant, index) => {
          const state = participantStates.get(participant.id) || {
            volume: 80,
            pan: 0,
            isMuted: false,
            isSoloed: false,
          };
          const color = participant.color || PARTICIPANT_COLORS[index % PARTICIPANT_COLORS.length];
          // In solo mode, non-soloed channels are effectively muted
          const effectivelyMuted = state.isMuted || (hasAnySolo && !state.isSoloed);

          return (
            <MixerChannel
              key={participant.id}
              id={participant.id}
              name={participant.name}
              type="peer"
              level={effectivelyMuted ? 0 : participant.level}
              volume={state.volume}
              pan={state.pan}
              isMuted={effectivelyMuted}
              isSoloed={state.isSoloed}
              color={color}
              onVolumeChange={handleParticipantVolumeChange}
              onPanChange={handleParticipantPanChange}
              onMuteToggle={handleParticipantMuteToggle}
              onSoloToggle={handleParticipantSoloToggle}
            />
          );
        })}

        {/* Spacer */}
        <div className="mixer-console__spacer" />

        {/* Master Channel */}
        <MixerChannel
          id="master"
          name={t("mixer.master", "Master")}
          type="master"
          level={masterLevel}
          volume={masterState.volume}
          pan={masterState.pan}
          isMuted={masterState.isMuted}
          color="#7c5cff"
          onVolumeChange={handleMasterVolumeChange}
          onPanChange={handleMasterPanChange}
          onMuteToggle={handleMasterMuteToggle}
        />
      </div>
    </div>
  );
}

export default MixerConsole;
