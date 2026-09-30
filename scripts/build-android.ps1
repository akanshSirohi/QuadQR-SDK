param(
    [string]$OutDir = "android-libs",
    [int]$Platform = 21,
    [switch]$Arm64Only
)

$ErrorActionPreference = "Stop"

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "Cargo/Rust is not installed. Install rustup first: https://rustup.rs"
}

if (-not (Get-Command cargo-ndk -ErrorAction SilentlyContinue)) {
    throw "cargo-ndk is not installed. Run: cargo install cargo-ndk"
}

$env:CARGO_NDK_PLATFORM = "$Platform"

if ($Arm64Only) {
    rustup target add aarch64-linux-android
    cargo ndk -t arm64-v8a -o $OutDir build --release -p quadqr-native
} else {
    rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
    cargo ndk `
        -t arm64-v8a `
        -t armeabi-v7a `
        -t x86_64 `
        -o $OutDir `
        build --release -p quadqr-native
}

Write-Host ""
Write-Host "QuadQR Android libraries generated in: $OutDir"
Get-ChildItem -Path $OutDir -Recurse -Filter "libquadqr.so" | ForEach-Object {
    Write-Host " - $($_.FullName)"
}
