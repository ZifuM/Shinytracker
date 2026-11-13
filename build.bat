@echo off
title Shiny Tracker build
cd /d "%~dp0"
del /q build-log.txt build-status.txt 2>nul
taskkill /im "Shiny Tracker.exe" /f >nul 2>&1
echo Building Shiny Tracker - the first build takes a few minutes...
echo (full output goes to build-log.txt)
rem Fresh Pokedex data from the Platinum Together Wiki (keeps the old copy if offline).
if not exist assets mkdir assets
powershell -NoProfile -Command "try { Invoke-WebRequest -UseBasicParsing -Uri 'https://platinumtogether.online/wiki/data/wiki.json' -OutFile 'assets\wiki.json.tmp' -TimeoutSec 30; Move-Item -Force 'assets\wiki.json.tmp' 'assets\wiki.json'; Write-Host 'Pokedex data updated.' } catch { Remove-Item -ErrorAction SilentlyContinue 'assets\wiki.json.tmp'; Write-Host 'Pokedex data not downloaded, using the copy already here.' }"
rem Bundle the C runtime into the .exe so it runs on any Windows 10/11 PC.
set RUSTFLAGS=-C target-feature=+crt-static
rem Always rebuild the app itself (libraries stay cached, so this is quick).
cargo clean --release -p shiny-tracker >nul 2>&1
cargo build --release > build-log.txt 2>&1
if errorlevel 1 (
  echo FAILED> build-status.txt
  echo.
  echo BUILD FAILED - last lines of build-log.txt:
  powershell -NoProfile -Command "Get-Content build-log.txt -Tail 40"
  pause
  exit /b 1
)
rem Close the app again in case it was reopened during the build, then replace it.
taskkill /im "Shiny Tracker.exe" /f >nul 2>&1
timeout /t 1 >nul
copy /y "target\release\shiny-tracker.exe" "Shiny Tracker.exe" >nul
rem Plain copy for sharing with friends.
if not exist share mkdir share
copy /y "target\release\shiny-tracker.exe" "share\Shiny Tracker.exe" >nul
rem ...and a fresh zip of it, ready to send.
powershell -NoProfile -Command "Compress-Archive -Path 'share\Shiny Tracker.exe' -DestinationPath 'share\Shiny Tracker.zip' -Force" >nul 2>&1
echo OK> build-status.txt
rem Say which version was built (the app shows the same number) and note it
rem next to the files to attach, so a release always gets the right build.
powershell -NoProfile -Command "$v = (Select-String -Path 'Cargo.toml' -Pattern '^version = .(.+).$' | Select-Object -First 1).Matches[0].Groups[1].Value; $v = $v -replace '-','.'; Set-Content 'share\version.txt' $v; Write-Host ''; Write-Host ('BUILD OK - Poke Together Companion v' + $v); Write-Host ('Attach share\Shiny Tracker.exe and .zip to the GitHub release v' + $v)"
echo Starting Shiny Tracker...
start "" "Shiny Tracker.exe"
timeout /t 6 >nul
