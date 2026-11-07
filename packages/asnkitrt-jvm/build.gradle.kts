// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

import com.vanniktech.maven.publish.JavadocJar
import com.vanniktech.maven.publish.KotlinJvm
import com.vanniktech.maven.publish.SourcesJar
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    kotlin("jvm") version "2.4.21"
    id("com.vanniktech.maven.publish") version "0.37.0"
}

group = "org.boxtheta"

repositories {
    mavenCentral()
}

dependencies {
    // CBOR for the .asni files, and the JSON of JER with the core it
    // brings.
    implementation("com.fasterxml.jackson.dataformat:jackson-dataformat-cbor:2.22.4")
    // The XML of XER, read with StAX.
    implementation("com.fasterxml.woodstox:woodstox-core:7.3.0")

    testImplementation(platform("org.junit:junit-bom:6.1.3"))
    testImplementation("org.junit.jupiter:junit-jupiter")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
    // To read the vectors, of which the numbers are of any size.
    testImplementation("com.fasterxml.jackson.core:jackson-databind:2.22.4")
}

// Java 17 and later, whatever the JDK it is built with.
java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
}

tasks.withType<JavaCompile>().configureEach {
    options.release = 17
}

kotlin {
    explicitApi()

    compilerOptions {
        jvmTarget = JvmTarget.JVM_17
        freeCompilerArgs.add("-Xjdk-release=17")
        allWarningsAsErrors = true
    }
}

tasks.test {
    useJUnitPlatform()
    // The tests read the vectors and the examples of the repository.
    systemProperty("asnkit.repository", rootDir.resolve("../..").canonicalPath)
    inputs.dir(rootDir.resolve("../../tests/vectors")).withPathSensitivity(PathSensitivity.RELATIVE)
    inputs.dir(rootDir.resolve("../../examples")).withPathSensitivity(PathSensitivity.RELATIVE)
    inputs.file(rootDir.resolve("../../docs/file_formats.md")).withPathSensitivity(PathSensitivity.RELATIVE)
    testLogging {
        events("failed")
        showStandardStreams = true
        exceptionFormat = org.gradle.api.tasks.testing.logging.TestExceptionFormat.FULL
    }
}

mavenPublishing {
    configure(KotlinJvm(javadocJar = JavadocJar.Empty(), sourcesJar = SourcesJar.Sources()))
    coordinates("org.boxtheta", "asnkitrt", version.toString())
    publishToMavenCentral()
    signAllPublications()

    pom {
        name = "asnkitrt"
        description = "Encode and decode ASN.1 with compiled specifications (.asni files) on the JVM"
        url = "https://github.com/boxtheta/asnkit"
        inceptionYear = "2025"

        licenses {
            license {
                name = "MIT License"
                url = "https://opensource.org/license/mit"
                distribution = "repo"
            }
        }

        developers {
            developer {
                id = "boxtheta"
                name = "BoxTheta"
                url = "https://github.com/boxtheta"
            }
        }

        scm {
            url = "https://github.com/boxtheta/asnkit"
            connection = "scm:git:https://github.com/boxtheta/asnkit.git"
            developerConnection = "scm:git:ssh://git@github.com/boxtheta/asnkit.git"
        }
    }
}
