cmake_minimum_required(VERSION 3.15)
project(slang_project)

find_package(slang 5.0.0)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

add_executable(rename rename/pass.cc)
target_link_libraries(rename PUBLIC slang::slang)