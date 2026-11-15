plugins {
    kotlin("jvm") version "2.2.20"
}

repositories {
    mavenCentral()
}

kotlin {
    jvmToolchain(17)
    sourceSets {
        main {
            kotlin.srcDir("../../apps/desktop/src-tauri/gen/android/app/src/main/java")
            kotlin.include("io/github/zocs/inoutnext/RuntimeAssets.kt")
        }
        test {
            kotlin.srcDir("../../apps/desktop/src-tauri/gen/android/app/src/test/java")
            kotlin.include("io/github/zocs/inoutnext/RuntimeAssetsTest.kt")
        }
    }
}

dependencies {
    testImplementation("junit:junit:4.13.2")
}

tasks.test {
    testLogging {
        events("failed", "passed")
    }
}
