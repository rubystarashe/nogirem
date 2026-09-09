cmake_minimum_required(VERSION 3.20)
project(input_guard_helper LANGUAGES CXX)

add_executable(input-guard-helper main.cpp)
target_compile_features(input-guard-helper PRIVATE cxx_std_20)
target_compile_definitions(
  input-guard-helper
  PRIVATE
    UNICODE
    _UNICODE
    WIN32_LEAN_AND_MEAN
    NOMINMAX
)

if(MSVC)
  target_compile_options(input-guard-helper PRIVATE /utf-8 /W4 /permissive-)
endif()
