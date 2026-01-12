#!/usr/bin/env python3
"""
PESQ Audio Quality Evaluation Script

Evaluates audio quality using ITU-T P.862 PESQ algorithm.
Returns MOS-LQO score (1.0 - 4.5 scale).

Usage:
    python evaluate.py --reference ref.wav --degraded deg.wav [--mode wb|nb]
    python evaluate.py --json '{"reference": "ref.wav", "degraded": "deg.wav"}'
"""

import argparse
import json
import sys
from pathlib import Path

import numpy as np

try:
    from pesq import pesq
    PESQ_AVAILABLE = True
except ImportError:
    PESQ_AVAILABLE = False

try:
    import soundfile as sf
    SOUNDFILE_AVAILABLE = True
except ImportError:
    SOUNDFILE_AVAILABLE = False

try:
    from scipy import signal
    SCIPY_AVAILABLE = True
except ImportError:
    SCIPY_AVAILABLE = False


def load_audio(path: str, target_sr: int = 16000) -> tuple[np.ndarray, int]:
    """Load audio file and resample if needed."""
    if not SOUNDFILE_AVAILABLE:
        raise ImportError("soundfile is required: pip install soundfile")

    data, sr = sf.read(path)

    # Convert to mono if stereo
    if len(data.shape) > 1:
        data = np.mean(data, axis=1)

    # Resample if needed
    if sr != target_sr and SCIPY_AVAILABLE:
        num_samples = int(len(data) * target_sr / sr)
        data = signal.resample(data, num_samples)
        sr = target_sr

    # Ensure float32
    data = data.astype(np.float32)

    return data, sr


def align_signals(ref: np.ndarray, deg: np.ndarray, sr: int) -> tuple[np.ndarray, np.ndarray, float]:
    """Align degraded signal to reference using cross-correlation."""
    if not SCIPY_AVAILABLE:
        # Simple truncation if scipy not available
        min_len = min(len(ref), len(deg))
        return ref[:min_len], deg[:min_len], 0.0

    # Cross-correlation to find delay
    correlation = signal.correlate(deg, ref, mode='full')
    lag = np.argmax(np.abs(correlation)) - len(ref) + 1

    latency_ms = abs(lag) / sr * 1000

    # Align signals
    if lag > 0:
        deg = deg[lag:]
    elif lag < 0:
        ref = ref[-lag:]

    # Truncate to same length
    min_len = min(len(ref), len(deg))
    return ref[:min_len], deg[:min_len], latency_ms


def calculate_pesq(reference_path: str, degraded_path: str, mode: str = 'wb') -> dict:
    """
    Calculate PESQ score between reference and degraded audio.

    Args:
        reference_path: Path to reference (original) audio file
        degraded_path: Path to degraded (received) audio file
        mode: 'wb' for wideband (16kHz) or 'nb' for narrowband (8kHz)

    Returns:
        dict with 'mos', 'latency_ms', 'error' fields
    """
    result = {
        'mos': None,
        'latency_ms': None,
        'error': None,
        'mode': mode,
    }

    if not PESQ_AVAILABLE:
        result['error'] = "pesq library not available: pip install pesq"
        return result

    try:
        # Determine target sample rate
        target_sr = 16000 if mode == 'wb' else 8000

        # Load audio files
        ref, ref_sr = load_audio(reference_path, target_sr)
        deg, deg_sr = load_audio(degraded_path, target_sr)

        if ref_sr != deg_sr:
            result['error'] = f"Sample rate mismatch: ref={ref_sr}, deg={deg_sr}"
            return result

        # Align signals and measure latency
        ref_aligned, deg_aligned, latency_ms = align_signals(ref, deg, ref_sr)
        result['latency_ms'] = round(latency_ms, 2)

        # Calculate PESQ
        mos = pesq(ref_sr, ref_aligned, deg_aligned, mode)
        result['mos'] = round(float(mos), 3)

    except Exception as e:
        result['error'] = str(e)

    return result


def calculate_correlation_mos(reference_path: str, degraded_path: str) -> dict:
    """
    Fallback quality measure using correlation when PESQ is not available.
    Maps correlation to approximate MOS scale.
    """
    result = {
        'mos': None,
        'latency_ms': None,
        'error': None,
        'mode': 'correlation',
    }

    try:
        ref, ref_sr = load_audio(reference_path, 48000)
        deg, deg_sr = load_audio(degraded_path, 48000)

        # Align and measure latency
        ref_aligned, deg_aligned, latency_ms = align_signals(ref, deg, ref_sr)
        result['latency_ms'] = round(latency_ms, 2)

        # Calculate Pearson correlation
        if len(ref_aligned) == 0:
            result['error'] = "Empty audio after alignment"
            return result

        correlation = np.corrcoef(ref_aligned, deg_aligned)[0, 1]

        # Map correlation to approximate MOS scale
        # correlation 1.0 -> MOS 4.5
        # correlation 0.9 -> MOS 4.0
        # correlation 0.7 -> MOS 3.0
        # correlation 0.5 -> MOS 2.0
        # correlation 0.0 -> MOS 1.0
        mos = 1.0 + 3.5 * max(0, correlation)
        result['mos'] = round(float(mos), 3)
        result['correlation'] = round(float(correlation), 4)

    except Exception as e:
        result['error'] = str(e)

    return result


def main():
    parser = argparse.ArgumentParser(description='PESQ Audio Quality Evaluation')
    parser.add_argument('--reference', '-r', help='Reference audio file path')
    parser.add_argument('--degraded', '-d', help='Degraded audio file path')
    parser.add_argument('--mode', '-m', choices=['wb', 'nb'], default='wb',
                       help='PESQ mode: wb (wideband 16kHz) or nb (narrowband 8kHz)')
    parser.add_argument('--json', '-j', help='JSON input with reference and degraded paths')
    parser.add_argument('--fallback', '-f', action='store_true',
                       help='Use correlation-based fallback if PESQ unavailable')

    args = parser.parse_args()

    # Parse input
    if args.json:
        try:
            params = json.loads(args.json)
            reference = params['reference']
            degraded = params['degraded']
            mode = params.get('mode', 'wb')
            fallback = params.get('fallback', False)
        except (json.JSONDecodeError, KeyError) as e:
            print(json.dumps({'error': f'Invalid JSON input: {e}'}))
            sys.exit(1)
    else:
        if not args.reference or not args.degraded:
            parser.error('--reference and --degraded are required (or use --json)')
        reference = args.reference
        degraded = args.degraded
        mode = args.mode
        fallback = args.fallback

    # Validate files exist
    if not Path(reference).exists():
        print(json.dumps({'error': f'Reference file not found: {reference}'}))
        sys.exit(1)
    if not Path(degraded).exists():
        print(json.dumps({'error': f'Degraded file not found: {degraded}'}))
        sys.exit(1)

    # Calculate PESQ
    if PESQ_AVAILABLE and not fallback:
        result = calculate_pesq(reference, degraded, mode)
    else:
        result = calculate_correlation_mos(reference, degraded)
        if not PESQ_AVAILABLE:
            result['warning'] = 'PESQ not available, using correlation fallback'

    # Output JSON result
    print(json.dumps(result))

    # Exit with error code if failed
    if result.get('error'):
        sys.exit(1)


if __name__ == '__main__':
    main()
