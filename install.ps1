$ErrorActionPreference = 'Stop'

# Determine version (default: latest)
$version = $env:VERSION
if (-not $version) {
    $version = "latest"
}

$target = "x86_64-pc-windows-msvc"

if ($version -eq "latest") {
    $url = "https://github.com/doggy8088/subembed/releases/latest/download/subembed-$target.zip"
} else {
    if (-not $version.StartsWith("v")) {
        $version = "v$version"
    }
    $url = "https://github.com/doggy8088/subembed/releases/download/$version/subembed-$target.zip"
}

$installDir = Join-Path $HOME ".subembed"
$binDir = Join-Path $installDir "bin"
$zipFile = Join-Path $env:TEMP "subembed.zip"

Write-Host "Downloading subembed from $url..."
Invoke-WebRequest -Uri $url -OutFile $zipFile -UseBasicParsing

Write-Host "Extracting..."
if (-not (Test-Path $binDir)) {
    New-Item -ItemType Directory -Path $binDir | Out-Null
}

# Expand-Archive overrides files if we force it
Expand-Archive -Path $zipFile -DestinationPath $binDir -Force
Remove-Item $zipFile -Force

# Add to User PATH if not already present
$path = [Environment]::GetEnvironmentVariable("PATH", "User")
$pathParts = $path -split ";"
if ($pathParts -notcontains $binDir) {
    Write-Host "Adding $binDir to User PATH..."
    [Environment]::SetEnvironmentVariable("PATH", $path + ";$binDir", "User")
    # Update current session PATH too
    $env:PATH = "$env:PATH;$binDir"
}

Write-Host "subembed installed successfully to $binDir\subembed.exe!"
Write-Host "You may need to restart your terminal shell for the PATH changes to take effect."
