plugins {
    id("com.android.library") version "8.10.1"
    id("org.jetbrains.kotlin.android") version "2.2.20"
}

group = "io.github.akanshsirohi"
version = "0.1.0"

android {
    namespace = "io.github.akanshsirohi.quadqr"
    compileSdk = 36

    defaultConfig {
        minSdk = 21
        consumerProguardFiles("consumer-rules.pro")

        ndk {
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64")
        }
    }

    sourceSets {
        getByName("main") {
            // Keep the public SDK sources in /android so Java and Kotlin docs/examples
            // and the packaged AAR always use the same implementation.
            java.srcDir("../../android/java")
            jniLibs.srcDir("../../android-libs")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

kotlin {
    sourceSets.named("main") {
        kotlin.srcDir("../../android/kotlin")
    }

    compilerOptions {
        jvmTarget.set(
            org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
        )
    }
}
