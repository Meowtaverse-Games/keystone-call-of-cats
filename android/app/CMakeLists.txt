# This dummy library makes Gradle package libc++_shared.so, which Bevy audio needs.
cmake_minimum_required(VERSION 3.22.1)
project(keystone_cc_android)
add_library(dummy SHARED src/main/cpp/dummy.cpp)
