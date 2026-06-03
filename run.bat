@echo off
title WorldForge 2
cd /d "%~dp0"

echo ============================================
echo   WorldForge 2 - Dev Launcher
echo ============================================
echo.

:: Check Node
where node >nul 2>&1
if errorlevel 1 (
    echo [ERROR] Node.js not found. Install from https://nodejs.org
    pause
    exit /b 1
)

:: Check Rust
where cargo >nul 2>&1
if errorlevel 1 (
    echo [ERROR] Rust/Cargo not found. Install from https://rustup.rs
    pause
    exit /b 1
)

:: Install/update npm dependencies
echo [1/3] Installing dependencies...
call npm install --prefer-offline --no-audit --no-fund 2>nul
if errorlevel 1 (
    echo [WARN] npm install had issues, continuing anyway...
)

:: Pull latest if git repo
where git >nul 2>&1
if not errorlevel 1 (
    echo [2/3] Checking for updates...
    git pull --ff-only 2>nul
    if not errorlevel 1 (
        echo      Updated to latest version.
    ) else (
        echo      Already up to date or no remote configured.
    )
) else (
    echo [2/3] Git not found, skipping update check.
)

:: Launch
echo [3/3] Starting WorldForge 2...
echo.
echo   Frontend: http://localhost:1420
echo   Press Ctrl+C to stop.
echo.
call npm run tauri dev
