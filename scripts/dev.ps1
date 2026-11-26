#Requires -Version 7.0
param(
    [ValidateSet("dev", "build", "test", "restore-taskbar")]
    [string]$Command = "dev"
)

$ErrorActionPreference = "Stop"
$RootDir = Split-Path -Parent $PSScriptRoot
Push-Location $RootDir

try {
    switch ($Command) {
        "dev" {
            npm run tauri:dev
        }
        "build" {
            npm run build
            npm run tauri:build
        }
        "test" {
            npx tsc --noEmit
            npm run build
            Push-Location src-tauri
            try {
                cargo test
            } finally {
                Pop-Location
            }
        }
        "restore-taskbar" {
            Push-Location src-tauri
            try {
                cargo run -- --restore-taskbar
            } finally {
                Pop-Location
            }
        }
    }
} finally {
    Pop-Location
}
