cmake_minimum_required(VERSION 3.22)

set(CMAKE_SYSTEM_NAME Generic)

get_filename_component(TOOLCHAIN_BIN_DIR ${CMAKE_C_COMPILER} DIRECTORY)
set(TOOLCHAIN_PREFIX "riscv-wch-elf-")

if (CMAKE_HOST_WIN32)
  set(HOST_EXEC_SUFFIX ".exe")
else ()
  set(HOST_EXEC_SUFFIX "")
endif ()

macro(find_toolchain_program VAR NAME)
  find_program(CMAKE_${VAR}
            NAMES "${TOOLCHAIN_PREFIX}${NAME}${HOST_EXEC_SUFFIX}"
            PATHS "${TOOLCHAIN_BIN_DIR}"
            NO_DEFAULT_PATH
    )
endmacro()

find_toolchain_program(C_COMPILER gcc)
find_toolchain_program(CXX_COMPILER g++)
find_toolchain_program(ASM_COMPILER gcc)
find_toolchain_program(OBJCOPY objcopy)
find_toolchain_program(OBJDUMP objdump)
find_toolchain_program(SIZE size)
find_toolchain_program(AR ar)

set(CMAKE_C_OUTPUT_EXTENSION .o)
set(CMAKE_CXX_OUTPUT_EXTENSION .o)
set(CMAKE_C_COMPILER_WORKS TRUE)
set(CMAKE_CXX_COMPILER_WORKS TRUE)
set(CMAKE_ASM_COMPILER_WORKS TRUE)

set(PROJECT_NAME "VoidPointer")
set(TARGET_ARTIFACT "${PROJECT_NAME}.elf")

project("${PROJECT_NAME}" LANGUAGES C ASM CXX)

include(FetchContent)
FetchContent_Declare(
        Corrosion
        GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
        GIT_TAG v0.6.1
)

set(Rust_CARGO_TARGET "riscv32imc-unknown-none-elf")
FetchContent_MakeAvailable(Corrosion)

corrosion_import_crate(
        MANIFEST_PATH core/Cargo.toml
        NO_STD
)

add_library(arch_config INTERFACE)

target_compile_options(arch_config INTERFACE
        -march=rv32imc_zba_zbb_zbc_zbs_xw
        -mabi=ilp32
        -mcmodel=medany
        -msmall-data-limit=8
        -mno-save-restore
        -fmax-errors=20
        -Os
        -fmessage-length=0
        -fsigned-char
        -ffunction-sections
        -fdata-sections
        -fno-common
        --param=highcode-gen-section-name=1
        -g
        $<$<COMPILE_LANGUAGE:C>: -std=gnu99>
        $<$<COMPILE_LANGUAGE:CXX>: -std=gnu++11 -fabi-version=0>
        $<$<COMPILE_LANGUAGE:ASM>: -x assembler-with-cpp>
)

target_link_options(arch_config INTERFACE
        -march=rv32imc_zba_zbb_zbc_zbs_xw
        -mabi=ilp32
        -T "${CMAKE_CURRENT_SOURCE_DIR}/platform/Ld/Link.ld"
        -nostartfiles
        -Wl,--gc-sections
        -Wl,--print-memory-usage
        -Wl,-Map=${PROJECT_NAME}.map
        --specs=nano.specs
        --specs=nosys.specs
)

target_compile_definitions(arch_config INTERFACE DEBUG=0)

file(GLOB_RECURSE PLATFORM_SOURCES
        "platform/Profile/*.c"
        "platform/APP/*.c"
        "platform/StdPeriphDriver/*.c"
        "platform/HAL/*.c"
        "platform/Startup/*.S"
        "platform/LIB/*.S"
        "platform/Bind/*.c"
)

add_executable(${TARGET_ARTIFACT} ${PLATFORM_SOURCES})

add_compile_options(-fmacro-prefix-map=${CMAKE_SOURCE_DIR}=..)

target_include_directories(${TARGET_ARTIFACT} PRIVATE
        "${CMAKE_CURRENT_SOURCE_DIR}/platform/Startup/"
        "${CMAKE_CURRENT_SOURCE_DIR}/platform/APP/include"
        "${CMAKE_CURRENT_SOURCE_DIR}/platform/Profile/include"
        "${CMAKE_CURRENT_SOURCE_DIR}/platform/StdPeriphDriver/inc"
        "${CMAKE_CURRENT_SOURCE_DIR}/platform/HAL/include"
        "${CMAKE_CURRENT_SOURCE_DIR}/platform/Ld/"
        "${CMAKE_CURRENT_SOURCE_DIR}/platform/LIB/"
        "${CMAKE_CURRENT_SOURCE_DIR}/platform/RVMSIS/"
        "${CMAKE_CURRENT_SOURCE_DIR}/platform/Bind/"
)

target_link_directories(${TARGET_ARTIFACT} PRIVATE
        "${CMAKE_CURRENT_SOURCE_DIR}/platform/"
        "${CMAKE_CURRENT_SOURCE_DIR}/platform/LIB/"
        "${CMAKE_CURRENT_SOURCE_DIR}/platform/StdPeriphDriver/"
)

target_link_libraries(${TARGET_ARTIFACT} PRIVATE
        arch_config
        void_pointer_core
        ISP585
        CH58xBLE
)

add_custom_command(TARGET ${TARGET_ARTIFACT} POST_BUILD
        COMMAND ${CMAKE_OBJCOPY} -O ihex "${TARGET_ARTIFACT}" "${PROJECT_NAME}.hex"
        COMMAND ${CMAKE_OBJDUMP} --source --all-headers --demangle -M xw --line-numbers --wide "${TARGET_ARTIFACT}" > "${PROJECT_NAME}.lst"
        COMMAND ${CMAKE_SIZE} --format=berkeley "${TARGET_ARTIFACT}"
)

get_filename_component(CARGO_MANIFEST_ABS "${CMAKE_CURRENT_SOURCE_DIR}/core/Cargo.toml" ABSOLUTE)
string(SHA1 CARGO_PATH_HASH "${CARGO_MANIFEST_ABS}")
string(SUBSTRING "${CARGO_PATH_HASH}" 0 5 CARGO_PATH_HASH)
set(CORROSION_TARGET_DIR "${CMAKE_BINARY_DIR}/cargo/core_${CARGO_PATH_HASH}")

file(MAKE_DIRECTORY "${CORROSION_TARGET_DIR}")

set(RUST_LOCAL_TARGET "${CMAKE_CURRENT_SOURCE_DIR}/core/target")

if(EXISTS "${RUST_LOCAL_TARGET}" AND NOT IS_SYMLINK "${RUST_LOCAL_TARGET}")
  message(WARNING "Please delete the existing 'core/target' directory manually so CMake can create a symlink.")
else()
  file(CREATE_LINK "${CORROSION_TARGET_DIR}" "${RUST_LOCAL_TARGET}" SYMBOLIC)
endif()
