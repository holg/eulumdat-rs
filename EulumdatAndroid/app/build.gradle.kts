import java.util.Base64
import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

// Load .env file from app directory
val envFile = file(".env")
val envProperties = Properties().apply {
    if (envFile.exists()) {
        envFile.inputStream().use { load(it) }
    }
}

fun getEnvProperty(key: String, default: String = ""): String {
    return envProperties.getProperty(key) ?: System.getenv(key) ?: default
}

android {
    namespace = "eu.trahe.eulumdat"
    compileSdk = 35

    // Name output files as "eulumdat-release.aab" instead of "app-release.aab"
    base.archivesName.set("eulumdat")

    defaultConfig {
        applicationId = getEnvProperty("APPLICATION_ID", "eu.trahe.eulumdat")
        minSdk = 26
        targetSdk = 35
        versionCode = getEnvProperty("VERSION_CODE", "1").toIntOrNull() ?: 1
        versionName = getEnvProperty("VERSION_NAME", "0.2.1")

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        vectorDrawables {
            useSupportLibrary = true
        }

        // Enable NDK for native libraries
        ndk {
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64", "x86")
        }
    }

    // Signing configuration - supports local file OR base64 (for CI)
    signingConfigs {
        create("release") {
            // Option 1: Local keystore file (for local development)
            val localKeystore = file("eulumdat.keystore")

            // Option 2: Base64 encoded keystore from .env (for CI/CD)
            val keystoreBase64 = getEnvProperty("ANDROID_KEYSTORE_BASE64")

            when {
                localKeystore.exists() -> {
                    // Use local keystore file
                    storeFile = localKeystore
                    storePassword = getEnvProperty("KEYSTORE_PASSWORD")
                    keyAlias = getEnvProperty("KEY_ALIAS", "eulumdat")
                    keyPassword = getEnvProperty("KEY_PASSWORD")
                }
                keystoreBase64.isNotEmpty() -> {
                    // Decode base64 keystore to temp file (CI/CD)
                    val keystoreFile = File.createTempFile("keystore", ".jks", buildDir)
                    keystoreFile.writeBytes(Base64.getDecoder().decode(keystoreBase64))
                    storeFile = keystoreFile
                    storePassword = getEnvProperty("KEYSTORE_PASSWORD")
                    keyAlias = getEnvProperty("KEY_ALIAS", "eulumdat")
                    keyPassword = getEnvProperty("KEY_PASSWORD")
                }
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
            // Use release signing if configured
            val releaseConfig = signingConfigs.findByName("release")
            if (releaseConfig?.storeFile != null) {
                signingConfig = releaseConfig
            }
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }

    kotlinOptions {
        jvmTarget = "1.8"
    }

    buildFeatures {
        compose = true
    }

    composeOptions {
        kotlinCompilerExtensionVersion = "1.5.4"
    }

    packaging {
        resources {
            excludes += "/META-INF/{AL2.0,LGPL2.1}"
        }
    }
}

dependencies {
    // Core Android
    implementation("androidx.core:core-ktx:1.12.0")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.6.2")
    implementation("androidx.activity:activity-compose:1.8.1")

    // Compose
    implementation(platform("androidx.compose:compose-bom:2023.10.01"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-graphics")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-extended")

    // Navigation
    implementation("androidx.navigation:navigation-compose:2.7.5")

    // File picking
    implementation("androidx.documentfile:documentfile:1.0.1")

    // Coil for SVG rendering
    implementation("io.coil-kt:coil-compose:2.5.0")
    implementation("io.coil-kt:coil-svg:2.5.0")

    // JNA for uniffi bindings
    implementation("net.java.dev.jna:jna:5.14.0@aar")

    // Testing
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.5")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.1")
    androidTestImplementation(platform("androidx.compose:compose-bom:2023.10.01"))
    androidTestImplementation("androidx.compose.ui:ui-test-junit4")
    debugImplementation("androidx.compose.ui:ui-tooling")
    debugImplementation("androidx.compose.ui:ui-test-manifest")
}
