cmake_minimum_required(VERSION 3.16)
project(iso-flasher C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_C_EXTENSIONS ON)

add_executable(iso-flasher iso-flasher.c)

if (CMAKE_C_COMPILER_ID MATCHES "GNU|Clang")
    target_compile_options(iso-flasher PRIVATE -Wall -Wextra -Wpedantic)
endif()

install(TARGETS iso-flasher RUNTIME DESTINATION bin)
