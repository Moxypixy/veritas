$ErrorActionPreference = "Stop"

$ArgentDir = if ($env:ARGENT_TEMPLATE_ARGENT_DIR) {
    $env:ARGENT_TEMPLATE_ARGENT_DIR
} else {
    "C:\Users\linft\argent-demos\argent"
}

if (-not (Test-Path (Join-Path $ArgentDir "Cargo.toml") -PathType Leaf)) {
    throw "Argent checkout not found at $ArgentDir. Set ARGENT_TEMPLATE_ARGENT_DIR to the local checkout."
}

$InsideWorkTree = (& git -C $ArgentDir rev-parse --is-inside-work-tree 2>$null)
if ($LASTEXITCODE -ne 0 -or $InsideWorkTree.Trim() -ne "true") {
    throw "$ArgentDir exists but is not an Argent Git checkout"
}

Write-Host "using Argent checkout at $ArgentDir"
Write-Host "HEAD: $((& git -C $ArgentDir rev-parse HEAD).Trim())"

& cargo build
if ($LASTEXITCODE -ne 0) {
    throw "cargo build failed with exit code $LASTEXITCODE"
}

& cargo run --quiet --bin counter
if ($LASTEXITCODE -ne 0) {
    throw "counter smoke failed with exit code $LASTEXITCODE"
}
