# Sync effectcraft to .88 without piping git archive (Windows tar pax headers break remote tar).
param(
    [string]$Dest = "vinez@192.168.1.88",
    [string]$RemoteDir = "~/tmp/effectcraft-pilot"
)
$ErrorActionPreference = "Stop"
$Root = Split-Path $PSScriptRoot -Parent
$Tar = Join-Path $env:TEMP ("effectcraft-sync-{0}.tar" -f [guid]::NewGuid().ToString("n"))
Push-Location $Root
try {
    git archive --format=tar HEAD -o $Tar
    ssh $Dest "rm -rf $RemoteDir && mkdir -p $RemoteDir"
    scp $Tar "${Dest}:/tmp/effectcraft-sync.tar"
    ssh $Dest "tar -xf /tmp/effectcraft-sync.tar -C $RemoteDir && rm -f /tmp/effectcraft-sync.tar"
    Write-Host "Synced to ${Dest}:${RemoteDir}"
} finally {
    Pop-Location
    Remove-Item -Force -ErrorAction SilentlyContinue $Tar
}
