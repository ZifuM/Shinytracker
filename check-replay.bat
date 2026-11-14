@echo off
rem Checks that the code compiles WITH Shiny Replay's recorder switched on
rem (the replay-rec feature), without closing a running Shiny Tracker.
if not "%~1"=="min" (start "" /min "%~f0" min & exit /b)
title Shiny Tracker code check (Shiny Replay recorder)
cd /d "%~dp0"
del /q check-replay-log.txt check-replay-status.txt 2>nul
set RUSTFLAGS=-C target-feature=+crt-static
cargo check --release --features replay-rec > check-replay-log.txt 2>&1
if errorlevel 1 (echo FAILED> check-replay-status.txt) else (echo OK> check-replay-status.txt)
exit
