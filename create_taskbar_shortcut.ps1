# Create Taskbar Shortcut for LocalMind
# This script creates a shortcut to start_localmind.bat and provides instructions for pinning it

$ErrorActionPreference = "Stop"

# Get the project root directory (where this script is located)
$ScriptPath = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = $ScriptPath

# Get Desktop path (handles OneDrive redirects and different Windows versions)
$DesktopPath = [Environment]::GetFolderPath('Desktop')
Write-Host "Desktop path detected: $DesktopPath" -ForegroundColor Gray

# Paths
$BatchFile = Join-Path $ProjectRoot "start_localmind.bat"
$ShortcutPath = Join-Path $DesktopPath "LocalMind.lnk"
$IconPath = Join-Path $ProjectRoot "localmind-rs\icons\icon.ico"

# Normalize paths (resolve any relative paths)
$BatchFile = [System.IO.Path]::GetFullPath($BatchFile)
$ShortcutPath = [System.IO.Path]::GetFullPath($ShortcutPath)
$IconPath = [System.IO.Path]::GetFullPath($IconPath)

# Check if batch file exists
if (-not (Test-Path $BatchFile)) {
    Write-Host "Error: start_localmind.bat not found at: $BatchFile" -ForegroundColor Red
    exit 1
}

# Check if icon exists, fallback to default if not
if (-not (Test-Path $IconPath)) {
    Write-Host "Warning: Icon not found at $IconPath, using default icon" -ForegroundColor Yellow
    $IconPath = "$env:SystemRoot\System32\shell32.dll"
    $IconIndex = 137  # Default application icon
} else {
    $IconIndex = 0
}

Write-Host "Creating shortcut..." -ForegroundColor Cyan
Write-Host "  Target: $BatchFile" -ForegroundColor Gray
Write-Host "  Shortcut: $ShortcutPath" -ForegroundColor Gray
Write-Host "  Icon: $IconPath" -ForegroundColor Gray

# Ensure Desktop directory exists
if (-not (Test-Path $DesktopPath)) {
    Write-Host "Error: Desktop directory does not exist: $DesktopPath" -ForegroundColor Red
}

# Create WScript Shell object
$WshShell = New-Object -ComObject WScript.Shell

# Create shortcut
try {
    $Shortcut = $WshShell.CreateShortcut($ShortcutPath)
    $Shortcut.TargetPath = $BatchFile
    $Shortcut.WorkingDirectory = $ProjectRoot
    $Shortcut.Description = "LocalMind - Privacy-focused knowledge management system"
    $Shortcut.IconLocation = "$IconPath,$IconIndex"
    $Shortcut.Save()
    Write-Host "Shortcut saved successfully!" -ForegroundColor Green
} catch {
    Write-Host "Error creating shortcut: $_" -ForegroundColor Red
    Write-Host "Error details: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "Trying alternative location..." -ForegroundColor Yellow
    # Try saving to project root instead
    $ShortcutPath = Join-Path $ProjectRoot "LocalMind.lnk"
    $ShortcutPath = [System.IO.Path]::GetFullPath($ShortcutPath)
    try {
        $Shortcut = $WshShell.CreateShortcut($ShortcutPath)
        $Shortcut.TargetPath = $BatchFile
        $Shortcut.WorkingDirectory = $ProjectRoot
        $Shortcut.Description = "LocalMind - Privacy-focused knowledge management system"
        $Shortcut.IconLocation = "$IconPath,$IconIndex"
        $Shortcut.Save()
        Write-Host "Shortcut created in project directory instead: $ShortcutPath" -ForegroundColor Yellow
    } catch {
        Write-Host "Failed to create shortcut in project directory too: $_" -ForegroundColor Red
        exit 1
    }
}

Write-Host "Shortcut created successfully!" -ForegroundColor Green
Write-Host "Location: $ShortcutPath" -ForegroundColor Green
Write-Host ""

# Instructions for pinning to taskbar
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "To pin to taskbar:" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "1. Right-click the shortcut on your Desktop" -ForegroundColor Yellow
Write-Host "2. Select 'Pin to taskbar'" -ForegroundColor Yellow
Write-Host ""
Write-Host "OR" -ForegroundColor Yellow
Write-Host ""
Write-Host "1. Drag the shortcut from Desktop to the taskbar" -ForegroundColor Yellow
Write-Host ""
Write-Host "The shortcut is ready to use!" -ForegroundColor Green

