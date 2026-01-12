# Setup virtual audio devices on Windows using VB-Audio Virtual Cable
#
# This script configures VB-Cable for E2E testing.
# Requires VB-CABLE to be installed: https://vb-audio.com/Cable/
#
# Usage: .\setup-virtual-audio-windows.ps1 [-Action check|list|status]

param(
    [string]$Action = "status"
)

$VBCableName = "CABLE Input"
$VBCableOutputName = "CABLE Output"

function Test-VBCable {
    Write-Host "Checking for VB-Cable installation..."

    # Check for VB-Cable in audio devices
    $devices = Get-WmiObject Win32_SoundDevice
    $vbCable = $devices | Where-Object { $_.Name -like "*CABLE*" -or $_.Name -like "*VB-Audio*" }

    if ($vbCable) {
        Write-Host "VB-Cable is installed:" -ForegroundColor Green
        $vbCable | ForEach-Object { Write-Host "  $($_.Name)" }
        return $true
    } else {
        Write-Host "VB-Cable is not installed" -ForegroundColor Yellow
        Write-Host ""
        Write-Host "Download from: https://vb-audio.com/Cable/"
        Write-Host "Install the driver and restart your computer"
        return $false
    }
}

function Get-AudioDevices {
    Write-Host "Available audio devices:"
    Write-Host ""

    # Use PowerShell to list audio devices
    try {
        Add-Type -AssemblyName System.Windows.Forms

        # Get playback devices
        Write-Host "Playback devices:" -ForegroundColor Cyan
        $playbackDevices = [System.Windows.Forms.SystemInformation]::new()
        Get-WmiObject Win32_SoundDevice | ForEach-Object {
            Write-Host "  $($_.Name)"
        }
    } catch {
        Write-Host "Could not enumerate audio devices via WMI"
        Write-Host "Using alternative method..."

        # Alternative: use Windows Audio API via command
        Get-ItemProperty "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\*" -ErrorAction SilentlyContinue |
            Select-Object -ExpandProperty DeviceState -ErrorAction SilentlyContinue
    }
}

function Set-DefaultDevice {
    param([string]$DeviceName, [string]$Type)

    Write-Host "Setting default $Type device to: $DeviceName"

    # This requires nircmd or similar tool
    # Download from: https://www.nirsoft.net/utils/nircmd.html

    if (Get-Command nircmd -ErrorAction SilentlyContinue) {
        if ($Type -eq "recording") {
            nircmd setdefaultsounddevice "$DeviceName" 2
        } else {
            nircmd setdefaultsounddevice "$DeviceName" 1
        }
        Write-Host "Default $Type device set to: $DeviceName" -ForegroundColor Green
    } else {
        Write-Host "nircmd not found. Install from: https://www.nirsoft.net/utils/nircmd.html" -ForegroundColor Yellow
        Write-Host ""
        Write-Host "Or manually set audio devices in:"
        Write-Host "  Settings > System > Sound > Input/Output"
    }
}

function Get-Status {
    Write-Host "Windows Audio Status:"
    Write-Host ""

    if (Test-VBCable) {
        Write-Host ""
        Get-AudioDevices
    }

    Write-Host ""
    Write-Host "Current default devices:" -ForegroundColor Cyan

    # Try to get current default devices
    try {
        $defaultPlayback = (Get-ItemProperty "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Audio\ActivePlaybackDevice" -ErrorAction SilentlyContinue).ActivePlaybackDevice
        $defaultRecording = (Get-ItemProperty "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Audio\ActiveRecordingDevice" -ErrorAction SilentlyContinue).ActiveRecordingDevice

        Write-Host "  Playback:  $defaultPlayback"
        Write-Host "  Recording: $defaultRecording"
    } catch {
        Write-Host "  (Could not determine default devices)"
    }
}

function Show-Usage {
    Write-Host "Usage: .\setup-virtual-audio-windows.ps1 [-Action <command>]"
    Write-Host ""
    Write-Host "Commands:"
    Write-Host "  check  - Check if VB-Cable is installed"
    Write-Host "  list   - List available audio devices"
    Write-Host "  status - Show current audio configuration (default)"
    Write-Host ""
    Write-Host "Example:"
    Write-Host "  .\setup-virtual-audio-windows.ps1 -Action check"
}

# Main script logic
switch ($Action.ToLower()) {
    "check" {
        Test-VBCable | Out-Null
    }
    "list" {
        Get-AudioDevices
    }
    "status" {
        Get-Status
    }
    "help" {
        Show-Usage
    }
    default {
        Show-Usage
        exit 1
    }
}
