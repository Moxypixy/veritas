@echo off
setlocal

if defined ARGENT_TEMPLATE_ARGENT_DIR (
    set "ARGENT_DIR=%ARGENT_TEMPLATE_ARGENT_DIR%"
) else (
    set "ARGENT_DIR=C:\Users\linft\argent-demos\argent"
)

if not exist "%ARGENT_DIR%\Cargo.toml" (
    echo error: Argent checkout not found at %ARGENT_DIR% 1>&2
    echo set ARGENT_TEMPLATE_ARGENT_DIR to the local Argent checkout 1>&2
    exit /b 1
)

cargo run --quiet --manifest-path "%ARGENT_DIR%\Cargo.toml" --bin argentc -- %*
exit /b %errorlevel%
