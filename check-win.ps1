$ErrorActionPreference = "Stop"

function Invoke-Native {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Command,

        [Parameter(ValueFromRemainingArguments = $true)]
        [string[]]$Arguments
    )

    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Command failed with exit code $LASTEXITCODE"
    }
}

Invoke-Native -Command "cargo" -Arguments @("fmt", "--all", "--check")
Invoke-Native -Command "cargo" -Arguments @("clippy", "--workspace", "--all-targets", "--all-features", "--", "-D", "warnings")
Invoke-Native -Command "cargo" -Arguments @("test", "--workspace", "--all-features")

if (Test-Path -Path "web" -PathType Container) {
    Push-Location "web"
    try {
        Invoke-Native -Command "npm" -Arguments @("run", "lint")
        Invoke-Native -Command "npm" -Arguments @("test")
    }
    finally {
        Pop-Location
    }
}

Invoke-Native -Command "git" -Arguments @("diff", "--check")
