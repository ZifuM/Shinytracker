@echo off
rem Checks that the code compiles, without closing a running Shiny Tracker.
if not "%~1"=="min" (start "" /min "%~f0" min & exit /b)
title Shiny Tracker code check
cd /d "%~dp0"
del /q check-log.txt check-status.txt 2>nul
set RUSTFLAGS=-C target-feature=+crt-static
cargo check --release > check-log.txt 2>&1
if errorlevel 1 (echo FAILED> check-status.txt) else (echo OK> check-status.txt)
exit
