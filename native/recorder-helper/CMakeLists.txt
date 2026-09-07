cmake_minimum_required(VERSION 3.20)
project(nogirem_recorder_helper LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_executable(recorder-helper main.cpp)

target_include_directories(recorder-helper BEFORE PRIVATE
  "${CMAKE_CURRENT_BINARY_DIR}/generated"
)

target_compile_definitions(recorder-helper PRIVATE
  UNICODE
  _UNICODE
  WIN32_LEAN_AND_MEAN
  NOMINMAX
)

target_link_libraries(recorder-helper PRIVATE
  d3d11
  dxgi
  windowsapp
  mf
  mfplat
  mfreadwrite
  mfuuid
  ole32
  shlwapi
)

if(MSVC)
  target_compile_options(recorder-helper PRIVATE /utf-8 /W4 /permissive- /await)
endif()
