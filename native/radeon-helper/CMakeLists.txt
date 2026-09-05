cmake_minimum_required(VERSION 3.20)
project(nogirem_radeon_helper LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(ADLX_SDK "${CMAKE_CURRENT_SOURCE_DIR}/../../third_party/adlx/SDK")

add_executable(radeon-helper
  main.cpp
  "${ADLX_SDK}/ADLXHelper/Windows/Cpp/ADLXHelper.cpp"
  "${ADLX_SDK}/Platform/Windows/WinAPIs.cpp"
)

target_include_directories(radeon-helper PRIVATE
  "${ADLX_SDK}/Include"
  "${ADLX_SDK}/ADLXHelper/Windows/Cpp"
)

target_compile_definitions(radeon-helper PRIVATE UNICODE _UNICODE)

if(MSVC)
  target_compile_options(radeon-helper PRIVATE /utf-8)
endif()
