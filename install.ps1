# Spotsie installer for Windows.
#
#   irm https://raw.githubusercontent.com/ahaan-shah/spotsie/main/install.ps1 | iex
#
# Downloads the latest installer, checks its SHA-256 against the release's
# checksums.txt, and installs Spotsie for the current user (no admin prompt).
#
# Environment:
#   SPOTSIE_VERSION   install a specific version (e.g. 0.1.1) instead of the latest
#
# Everything runs inside a script block so nothing leaks into your session,
# and errors are thrown rather than calling `exit`, which would close it.
& {
    $ErrorActionPreference = 'Stop'
    $ProgressPreference = 'SilentlyContinue'  # Invoke-WebRequest is far faster without the progress bar
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

    $repo = 'ahaan-shah/spotsie'
    if ($env:SPOTSIE_VERSION) {
        $tag = 'v' + $env:SPOTSIE_VERSION.TrimStart('v')
    } else {
        $tag = (Invoke-RestMethod -UseBasicParsing -Uri "https://api.github.com/repos/$repo/releases/latest").tag_name
    }
    $base = "https://github.com/$repo/releases/download/$tag"

    $arch = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64' -or $env:PROCESSOR_ARCHITEW6432 -eq 'ARM64') { 'aarch64' } else { 'x86_64' }
    $asset = "spotsie-$tag-$arch-pc-windows-msvc-setup.exe"
    $tmp = Join-Path ([IO.Path]::GetTempPath()) ("spotsie-" + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $tmp | Out-Null
    try {
        Write-Host "spotsie: downloading $asset"
        Invoke-WebRequest -UseBasicParsing -Uri "$base/$asset" -OutFile (Join-Path $tmp $asset)

        $sums = Join-Path $tmp 'checksums.txt'
        Invoke-WebRequest -UseBasicParsing -Uri "$base/checksums.txt" -OutFile $sums
        $line = Get-Content $sums | Where-Object { ($_ -split '\s+')[-1].TrimStart('*') -eq $asset } | Select-Object -First 1
        if (-not $line) { throw "$asset isn't listed in the release's checksums" }
        $expected = ($line -split '\s+')[0].ToLowerInvariant()
        $actual = (Get-FileHash -Algorithm SHA256 (Join-Path $tmp $asset)).Hash.ToLowerInvariant()
        if ($expected -ne $actual) { throw "checksum mismatch for $asset" }
        Write-Host "spotsie: checksum ok"

        Write-Host "spotsie: installing"
        $setupArgs = '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/CLOSEAPPLICATIONS'
        $p = Start-Process -FilePath (Join-Path $tmp $asset) -ArgumentList $setupArgs -Wait -PassThru
        if ($p.ExitCode -ne 0) { throw "the installer exited with code $($p.ExitCode)" }
        Write-Host "spotsie: installed Spotsie $($tag.TrimStart('v')). Open it from the Start menu."
    } finally {
        Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
    }
}
