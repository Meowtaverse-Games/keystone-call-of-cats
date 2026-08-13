# Android SDK/NDK tools are distributed for x86_64 Linux. Docker therefore runs
# this image as linux/amd64; Docker Desktop/binfmt handles it on ARM64 hosts.
FROM --platform=linux/amd64 rust:1.90.0-bookworm

ARG ANDROID_CMDLINE_TOOLS_VERSION=13114758
ARG ANDROID_NDK_VERSION=27.2.12479018
ARG GRADLE_VERSION=8.10.2

ENV ANDROID_HOME=/opt/android-sdk \
    ANDROID_NDK_HOME=/opt/android-sdk/ndk/${ANDROID_NDK_VERSION} \
    DEBIAN_FRONTEND=noninteractive \
    PATH=/opt/android-sdk/cmdline-tools/latest/bin:/opt/gradle/bin:${PATH}

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl openjdk-17-jdk unzip \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p ${ANDROID_HOME}/cmdline-tools /opt/gradle /cargo \
    && chmod 0777 /cargo \
    && curl --fail --location --retry 3 --output /tmp/android-tools.zip "https://dl.google.com/android/repository/commandlinetools-linux-${ANDROID_CMDLINE_TOOLS_VERSION}_latest.zip" \
    && unzip -q /tmp/android-tools.zip -d ${ANDROID_HOME}/cmdline-tools \
    && mv ${ANDROID_HOME}/cmdline-tools/cmdline-tools ${ANDROID_HOME}/cmdline-tools/latest \
    && rm /tmp/android-tools.zip \
    && yes | sdkmanager --licenses \
    && sdkmanager "platform-tools" "platforms;android-35" "build-tools;35.0.0" "ndk;${ANDROID_NDK_VERSION}" "cmake;3.22.1" \
    && curl --fail --location --retry 3 --output /tmp/gradle.zip "https://services.gradle.org/distributions/gradle-${GRADLE_VERSION}-bin.zip" \
    && unzip -q /tmp/gradle.zip -d /opt \
    && mv /opt/gradle-${GRADLE_VERSION}/* /opt/gradle/ \
    && rm /tmp/gradle.zip \
    && rustup target add aarch64-linux-android x86_64-linux-android

WORKDIR /workspace
