plugins {
    id("org.jetbrains.kotlin.multiplatform")
    id("com.android.kotlin.multiplatform.library")
}

kotlin {
    android {
        namespace = "org.cuenora.shared"
        compileSdk = 36
        minSdk = 23
        withHostTest {}
        withDeviceTest {
            instrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
            execution = "HOST"
        }
        compilerOptions {
            jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
        }
    }

    listOf(iosArm64(), iosSimulatorArm64()).forEach { target ->
        target.binaries.framework {
            baseName = "CuenoraShared"
            isStatic = true
        }
    }

    sourceSets {
        val ktorVersion = "3.6.1"
        commonMain.dependencies {
            api("org.jetbrains.kotlinx:kotlinx-serialization-json:1.11.0")
            api("io.ktor:ktor-client-core:$ktorVersion")
            implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.11.0")
        }
        androidMain.dependencies { implementation("io.ktor:ktor-client-android:$ktorVersion") }
        iosMain.dependencies { implementation("io.ktor:ktor-client-darwin:$ktorVersion") }
        commonTest.dependencies {
            implementation(kotlin("test"))
            implementation("io.ktor:ktor-client-mock:$ktorVersion")
            implementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.11.0")
        }
        getByName("androidDeviceTest").dependencies {
            implementation(kotlin("test"))
            implementation("androidx.test:runner:1.7.0")
            implementation("androidx.test.ext:junit:1.3.0")
        }
    }
}
