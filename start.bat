@echo off
setlocal EnableDelayedExpansion
title AetherContracts Neural Link
color 0A

:: Check if Python is installed
python --version >nul 2>&1
if %errorlevel% neq 0 (
    echo [!] ERROR: Python is not installed or not in your PATH.
    echo [i] Please install Python 3.10+ to run the Aether launcher.
    pause
    exit /b
)

:: Run the expansive Python launcher
python scripts\launcher.py

pause
