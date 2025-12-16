@echo off
REM Build script for MikuDB on Windows
REM This script sets up the necessary environment variables for compiling zstd-sys

setlocal EnableDelayedExpansion

echo.
echo MikuDB Build Script
echo ===================

REM Use forward slashes and wrap each path in quotes for Clang
set "MSVC_PATH=C:/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC/14.40.33807/include"
set "SDK_PATH=C:/Program Files (x86)/Windows Kits/10/Include/10.0.22621.0/ucrt"

REM Check if paths exist (convert back to backslash for Windows check)
if not exist "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.40.33807\include" (
    echo WARNING: MSVC include path not found
    echo Please update the path in this script
)
if not exist "C:\Program Files (x86)\Windows Kits\10\Include\10.0.22621.0\ucrt" (
    echo WARNING: SDK include path not found
    echo Please update the path in this script
)

echo MSVC Include: %MSVC_PATH%
echo SDK Include:  %SDK_PATH%
echo.

REM The key is to have each -I"path" as a separate quoted argument
set BINDGEN_EXTRA_CLANG_ARGS="-I%MSVC_PATH%" "-I%SDK_PATH%"

echo BINDGEN_EXTRA_CLANG_ARGS=%BINDGEN_EXTRA_CLANG_ARGS%
echo.

cargo build --release %*

endlocal
