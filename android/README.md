# QuadQR Android

QuadQR Android exposes the Rust-powered QuadQR engine through a developer-friendly Android API. Application code works with normal classes, enums, option builders, byte arrays and result objects. You do not need to build option strings or parse engine responses yourself.

## Public API

Use `QuadQR.getInstance()` from Java or Kotlin. The low-level bridge is internal and is not part of the supported application API.

```java
QuadQR quadQR = QuadQR.getInstance();
EncodedQr code = quadQR.encodeText("Hello Android");
```

The Android API includes encoding, matrix decoding, image and RGBA scanning, rendering, logos, compression, encryption, signing, capacity tools, print guidance and reliability tools.

## Documentation

- [Java developer guide](JAVA.md)
- [Kotlin developer guide](KOTLIN.md)
- [Flutter developer guide](../packages/quadqr-flutter/README.md)

## Native library placement

After building the Android native libraries, keep the ABI folders together under your Android library or application `jniLibs` directory:

```text
src/main/jniLibs/
├── arm64-v8a/libquadqr.so
├── armeabi-v7a/libquadqr.so
└── x86_64/libquadqr.so
```

For development on a modern physical Android device, `arm64-v8a` is usually enough. Build all ABIs before distributing the library.


## Windows Java runtime

The Windows AAR build scripts use Android Studio's bundled JBR when it is compatible with the pinned Gradle version:

```text
C:\Program Files\Android\Android Studio\jbr
```

You do not need to add this Java installation to the global `PATH` or replace your system Java. `build-aar.bat` and `build-android-release.bat` select a JDK 17–23 before Gradle starts and print the detected Java version. If Studio's JBR is newer, install JDK 17 or 21 and set `QUADQR_JAVA_HOME`.

If Android Studio is installed elsewhere, set `QUADQR_JAVA_HOME` before building:

```bat
set QUADQR_JAVA_HOME=D:\Tools\Android Studio\jbr
build-aar.bat
```

The build runs on JDK 21 while the Android library intentionally continues to target Java 17 bytecode for wider Android/Gradle compatibility.

## Build the Android library

The repository now includes one-click build files at the repository root.

### Windows

Build all three Android native libraries:

```bat
build-android.bat
```

Package already-built native libraries into the developer AAR:

```bat
build-aar.bat
```

Build everything from Rust through the final AAR in one command:

```bat
build-android-release.bat
```

The complete release command produces:

```text
android-libs/
├── arm64-v8a/libquadqr.so
├── armeabi-v7a/libquadqr.so
└── x86_64/libquadqr.so

dist/
└── quadqr-android-release.aar
```

The AAR contains the object-oriented Java/Kotlin API and all three Android architectures. Application developers only add the AAR to their project and use `QuadQR`, `EncodeOptions`, `ScanOptions`, `RenderOptions`, and the other public SDK classes documented in the Java and Kotlin guides.

Gradle does not need to be installed globally. The AAR build script downloads the pinned Gradle version into the repository-local `.tools` directory the first time it is needed. Android Gradle dependencies are also resolved automatically on the first build.

### macOS / Linux

Equivalent commands are provided:

```bash
./build-android.sh
./build-aar.sh
./build-android-release.sh
```

### Requirements

For the complete release build, install:

- Rust through `rustup`
- `cargo-ndk` with `cargo install cargo-ndk`
- Android SDK and Android NDK
- JDK 17–23 (21 recommended for Gradle 8.11.1). On Windows the build helper uses a compatible Android Studio JBR when available.

The native build scripts add the required Rust Android targets automatically.
