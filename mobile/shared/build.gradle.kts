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
        }
        androidMain.dependencies { implementation("io.ktor:ktor-client-android:$ktorVersion") }
        iosMain.dependencies { implementation("io.ktor:ktor-client-darwin:$ktorVersion") }
        commonTest.dependencies {
            implementation(kotlin("test"))
            implementation("io.ktor:ktor-client-mock:$ktorVersion")
            implementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.11.0")
        }
    }
}
