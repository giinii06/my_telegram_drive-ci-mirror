plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

val tdriveVersionName: String = (project.findProperty("tdriveVersionName") as String?) ?: "0.0.0"
val tdriveVersionCode: Int = ((project.findProperty("tdriveVersionCode") as String?) ?: "1").toInt()

android {
    namespace = "dev.tdrive.server"
    compileSdk = 34

    defaultConfig {
        applicationId = "dev.tdrive.server"
        minSdk = 24
        targetSdk = 34
        versionCode = tdriveVersionCode
        versionName = tdriveVersionName
    }

    sourceSets {
        getByName("main") {
            // build-apk.sh drops the Go server here; binaries stay out of the
            // source tree (see .gitignore).
            jniLibs.srcDir(rootProject.file("jniLibs"))
        }
    }

    buildTypes {
        debug {
            isMinifyEnabled = false
        }
        release {
            isMinifyEnabled = false
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    packaging {
        jniLibs {
            // The service execs libtdrive.so from nativeLibraryDir, so it must
            // be extracted instead of loaded straight from the APK.
            useLegacyPackaging = true
        }
    }

    buildFeatures {
        buildConfig = false
    }
}

dependencies {
}
