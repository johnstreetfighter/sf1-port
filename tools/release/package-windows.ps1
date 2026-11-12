# Package an already-built, exact-tag Windows release. Run from PowerShell.
# Example: rtk proxy powershell -NoProfile -ExecutionPolicy Bypass -File tools/release/package-windows.ps1 -Tag v1.2.1 -BuildDirectory target-netplay/release -OutputDirectory target-netplay/packages
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][ValidatePattern('^v[0-9]+\.[0-9]+\.[0-9]+$')][string]$Tag,
    [Parameter(Mandatory = $true)][string]$BuildDirectory,
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [string]$SourceRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
)

$ErrorActionPreference = 'Stop'
$npRtk = Get-Command rtk -CommandType Application -ErrorAction SilentlyContinue
function Invoke-ReleaseCommand([string]$Command) {
    if ($npRtk) { & $npRtk.Source proxy $Command @args }
    else { & $Command @args }
}
function Get-Sha256([string]$Path) {
    $npSha = [Security.Cryptography.SHA256]::Create()
    $npStream = [IO.File]::OpenRead($Path)
    try { return ([BitConverter]::ToString($npSha.ComputeHash($npStream))).Replace('-', '').ToLowerInvariant() }
    finally { $npStream.Dispose(); $npSha.Dispose() }
}
$npSource = (Resolve-Path -LiteralPath $SourceRoot).Path
$npBuild = (Resolve-Path -LiteralPath $BuildDirectory).Path
$npExe = Join-Path $npBuild 'sf-game.exe'
if (-not (Test-Path -LiteralPath $npExe -PathType Leaf)) { throw 'BuildDirectory must contain sf-game.exe.' }

Push-Location -LiteralPath $npSource
try {
    $npHead = (Invoke-ReleaseCommand git rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'Cannot read the source revision.' }
    $npTagged = (Invoke-ReleaseCommand git rev-parse "$Tag^{commit}").Trim()
    if ($LASTEXITCODE -ne 0 -or $npHead -ne $npTagged) { throw 'Source HEAD must equal the release tag.' }
    Invoke-ReleaseCommand git diff --quiet HEAD --
    if ($LASTEXITCODE -ne 0) { throw 'Commit tracked source changes before packaging.' }

    # Only metadata is read here; builds must have completed separately.
    $npPreviousTarget = $env:CARGO_TARGET_DIR
    try {
        $env:CARGO_TARGET_DIR = Split-Path -Parent $npBuild
        $npMetadataText = Invoke-ReleaseCommand cargo metadata --locked --format-version 1 --filter-platform x86_64-pc-windows-msvc
        if ($LASTEXITCODE -ne 0) { throw 'Cannot resolve locked dependency metadata.' }
        $npMetadata = $npMetadataText | ConvertFrom-Json
    } finally { $env:CARGO_TARGET_DIR = $npPreviousTarget }
} finally { Pop-Location }

$npBanner = 'This historical baseline predates the workspace version and --version flag; its Cargo packages are 0.1.0. The v1.0.0 tag and source commit identify this release.'
if ([Version]$Tag.Substring(1) -ge [Version]'1.1.0') {
    # PowerShell can return before a GUI executable closes its output pipe.
    $npStart = [Diagnostics.ProcessStartInfo]::new($npExe, '--version')
    $npStart.UseShellExecute = $false
    $npStart.CreateNoWindow = $true
    $npStart.RedirectStandardOutput = $true
    $npVersion = [Diagnostics.Process]::Start($npStart)
    try {
        $npBanner = $npVersion.StandardOutput.ReadToEnd().Trim()
        $npVersion.WaitForExit()
        $npVersionExit = $npVersion.ExitCode
    } finally { $npVersion.Dispose() }
    if ($npVersionExit -ne 0 -or -not $npBanner.Contains($Tag) -or -not $npBanner.Contains($npHead.Substring(0, 8))) {
        throw 'Executable --version must identify both the requested version and the exact source revision.'
    }
}
$npRust = (Invoke-ReleaseCommand rustc --version | Out-String).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Cannot identify the Rust compiler.' }

$npOutput = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $npOutput -Force | Out-Null
$npName = "sf1-port-$Tag-windows-x86_64"
$npPackage = Join-Path $npOutput $npName
$npZip = "$npPackage.zip"
if (Test-Path -LiteralPath $npPackage) { throw "Package directory already exists: $npPackage" }
if (Test-Path -LiteralPath $npZip) { throw "Package archive already exists: $npZip" }
New-Item -ItemType Directory -Path $npPackage | Out-Null
Copy-Item -LiteralPath $npExe -Destination $npPackage
Copy-Item -LiteralPath (Join-Path $npSource 'shaders') -Destination $npPackage -Recurse
foreach ($npFile in @('README.md', 'THIRD_PARTY_NOTICES.md', 'Cargo.lock')) {
    Copy-Item -LiteralPath (Join-Path $npSource $npFile) -Destination $npPackage
}
if (Test-Path -LiteralPath (Join-Path $npSource 'docs/netplay.md')) {
    New-Item -ItemType Directory -Path (Join-Path $npPackage 'docs') | Out-Null
    Copy-Item -LiteralPath (Join-Path $npSource 'docs/netplay.md') -Destination (Join-Path $npPackage 'docs')
}

# Keep each dependency's own license/notice files, alongside an index identifying
# the locked package and public source. Include transitive/build dependencies;
# retaining extra workspace dependencies is harmless and makes the audit simple.
$npLicenseRoot = Join-Path $npPackage 'licenses/rust-dependencies'
New-Item -ItemType Directory -Path $npLicenseRoot -Force | Out-Null
$npLicenseIndex = [Collections.Generic.List[string]]::new()
$npLicenseIndex.Add('# Locked Rust dependency notices')
$npLicenseIndex.Add('')
$npLicenseIndex.Add('The executable uses default sf-game features, including statically linked libopenmpt. Cargo.lock records exact package identities; the files here retain dependency license and notice texts. The complete shaders retain their own headers and license files. Source for this game is linked in BUILD.txt; registry dependency source is available at the package links below.')
$npLicenseIndex.Add('')
$npNoticeCache = @{}
function Get-UpstreamNotices($Dependency, [string]$ManifestFolder) {
    $npVcsFile = Join-Path $ManifestFolder '.cargo_vcs_info.json'
    if (-not (Test-Path -LiteralPath $npVcsFile)) { throw "No license text or pinned upstream revision for $($Dependency.name)." }
    $npVcs = Get-Content -LiteralPath $npVcsFile -Raw | ConvertFrom-Json
    $npRevision = $npVcs.git.sha1
    if ($npRevision -notmatch '^[0-9a-f]{40}$') { throw "Missing pinned upstream revision for $($Dependency.name)." }
    $npRepository = [Uri]$Dependency.repository
    $npRepoPath = $npRepository.AbsolutePath.Trim('/').Replace('.git', '')
    $npCacheKey = "$npRepository@$npRevision"
    if ($npNoticeCache.ContainsKey($npCacheKey)) { return $npNoticeCache[$npCacheKey] }
    if ($npRepository.Host -eq 'github.com') {
        $npEntries = Invoke-ReleaseCommand gh api "repos/$npRepoPath/contents?ref=$npRevision" | ConvertFrom-Json
        if ($LASTEXITCODE -ne 0) { throw "Cannot read pinned upstream notices for $($Dependency.name)." }
    } elseif ($npRepository.Host -eq 'gitlab.com') {
        $npProject = [Uri]::EscapeDataString($npRepoPath)
        $npApi = "https://gitlab.com/api/v4/projects/$npProject/repository"
        $npEntries = Invoke-RestMethod -Uri "$npApi/tree?ref=$npRevision&per_page=100" -TimeoutSec 30
    } else { throw "No supported pinned license source for $($Dependency.name)." }
    $npEntries = @($npEntries | Where-Object { $_.name -match '^(LICENSES?|LICENCES?|COPYING|NOTICE|UNLICENSE|COPYRIGHT)([._-].*)?$' })
    $npFolders = [Collections.Generic.Stack[string]]::new()
    foreach ($npEntry in @($npEntries | Where-Object { $_.type -in @('dir', 'tree') })) { $npFolders.Push($npEntry.path) }
    $npEntries = @($npEntries | Where-Object { $_.type -in @('file', 'blob') })
    while ($npFolders.Count) {
        $npFolderPath = $npFolders.Pop()
        if ($npRepository.Host -eq 'github.com') {
            $npChildren = Invoke-ReleaseCommand gh api "repos/$npRepoPath/contents/$npFolderPath`?ref=$npRevision" | ConvertFrom-Json
            if ($LASTEXITCODE -ne 0) { throw "Cannot read pinned upstream license folder for $($Dependency.name)." }
        } else {
            $npFolderEncoded = [Uri]::EscapeDataString($npFolderPath)
            $npChildren = Invoke-RestMethod -Uri "$npApi/tree?path=$npFolderEncoded&ref=$npRevision&per_page=100" -TimeoutSec 30
        }
        foreach ($npChild in $npChildren) {
            if ($npChild.type -in @('dir', 'tree')) { $npFolders.Push($npChild.path) }
            elseif ($npChild.type -in @('file', 'blob')) { $npEntries += $npChild }
        }
    }
    if (-not $npEntries.Count) {
        # This published crate and its complete pinned tree were checked. Retain
        # their actual MIT declaration and source; do not invent a notice.
        if ($Dependency.name -eq 'vec_extract_if_polyfill' -and $Dependency.version -eq '0.1.0' -and $npRevision -eq '11267b12cc4ec3350026bd496654eb17746eaed6') {
            $npMissingNotice = 'The original Cargo.toml declares license = "MIT". This published crate and pinned upstream revision provide no separate license text, copyright notice or author field. The original manifests and complete src tree are retained verbatim in original-source. This records the upstream material; it does not add a copyright notice or new grant.'
            return [PSCustomObject]@{ Name = 'UPSTREAM-NOTICE-STATUS.txt'; Bytes = [Text.Encoding]::UTF8.GetBytes($npMissingNotice); Source = "$npRepository (commit $npRevision)"; MissingNotice = $true }
        }
        throw "Pinned upstream has no root license files for $($Dependency.name)."
    }
    $npNotices = @()
    foreach ($npEntry in $npEntries) {
        if ($npRepository.Host -eq 'github.com') {
            $npFile = Invoke-ReleaseCommand gh api "repos/$npRepoPath/contents/$($npEntry.path)?ref=$npRevision" | ConvertFrom-Json
            if ($LASTEXITCODE -ne 0) { throw "Cannot download pinned notice $($npEntry.name)." }
        } else {
            $npEntryPath = [Uri]::EscapeDataString($npEntry.path)
            $npFile = Invoke-RestMethod -Uri "$npApi/files/$npEntryPath`?ref=$npRevision" -TimeoutSec 30
        }
        $npNotices += [PSCustomObject]@{ Name = $npEntry.path; Bytes = [Convert]::FromBase64String($npFile.content); Source = "$npRepository (commit $npRevision)" }
    }
    $npNoticeCache[$npCacheKey] = $npNotices
    return $npNotices
}
foreach ($npDependency in ($npMetadata.packages | Where-Object { $_.source } | Sort-Object name, version)) {
    $npDepName = "$($npDependency.name)-$($npDependency.version)"
    $npDepFolder = Join-Path $npLicenseRoot $npDepName
    New-Item -ItemType Directory -Path $npDepFolder -Force | Out-Null
    $npManifestFolder = Split-Path -Parent $npDependency.manifest_path
    $npLicenseFiles = @(Get-ChildItem -LiteralPath $npManifestFolder -File -Recurse | Where-Object {
        $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE|UNLICENSE|COPYRIGHT)([._-].*)?$' -or $_.FullName -match '[\\/](LICENSES|LICENCES)[\\/]'
    })
    foreach ($npLicenseFile in $npLicenseFiles) {
        $npRelativeNotice = $npLicenseFile.FullName.Substring($npManifestFolder.Length + 1)
        $npNoticeDestination = Join-Path $npDepFolder $npRelativeNotice
        New-Item -ItemType Directory -Path (Split-Path -Parent $npNoticeDestination) -Force | Out-Null
        Copy-Item -LiteralPath $npLicenseFile.FullName -Destination $npNoticeDestination
    }
    $npRootLicenses = @($npLicenseFiles | Where-Object { $_.DirectoryName -eq $npManifestFolder -and $_.Name -match '^(LICENSE|LICENCE|COPYING|UNLICENSE)([._-].*)?$' })
    $npHasLicenseFolder = @($npLicenseFiles | Where-Object { $_.FullName.StartsWith((Join-Path $npManifestFolder 'LICENSES') + '\') -or $_.FullName.StartsWith((Join-Path $npManifestFolder 'LICENCES') + '\') }).Count -gt 0
    if (-not $npRootLicenses.Count -and -not $npHasLicenseFolder -and -not $npDependency.license_file) {
        $npUpstreamNotices = @(Get-UpstreamNotices $npDependency $npManifestFolder)
        foreach ($npNotice in $npUpstreamNotices) {
            $npPinnedNotice = Join-Path $npDepFolder $npNotice.Name
            New-Item -ItemType Directory -Path (Split-Path -Parent $npPinnedNotice) -Force | Out-Null
            [IO.File]::WriteAllBytes($npPinnedNotice, $npNotice.Bytes)
        }
        [IO.File]::WriteAllText((Join-Path $npDepFolder 'UPSTREAM-SOURCE.txt'), $npUpstreamNotices[0].Source)
        if ($npUpstreamNotices[0].MissingNotice) {
            $npOriginalSource = Join-Path $npDepFolder 'original-source'
            New-Item -ItemType Directory -Path $npOriginalSource | Out-Null
            foreach ($npManifestName in @('Cargo.toml', 'Cargo.toml.orig')) {
                $npOriginalManifest = Join-Path $npManifestFolder $npManifestName
                if (Test-Path -LiteralPath $npOriginalManifest) { Copy-Item -LiteralPath $npOriginalManifest -Destination $npOriginalSource }
            }
            Copy-Item -LiteralPath (Join-Path $npManifestFolder 'src') -Destination $npOriginalSource -Recurse
        }
    }
    if ($npDependency.license_file) {
        $npExplicitLicense = if ([IO.Path]::IsPathRooted($npDependency.license_file)) {
            $npDependency.license_file
        } else { Join-Path $npManifestFolder $npDependency.license_file }
        if (Test-Path -LiteralPath $npExplicitLicense -PathType Leaf) {
            Copy-Item -LiteralPath $npExplicitLicense -Destination $npDepFolder -Force
        }
    }
    $npLicenseIndex.Add("- $npDepName; license: $($npDependency.license); source: https://crates.io/crates/$($npDependency.name)/$($npDependency.version)")
}
[IO.File]::WriteAllLines((Join-Path $npPackage 'licenses/README.md'), $npLicenseIndex)

$npBuildInfo = @(
    "Street Fighter: The World Tournament - $Tag Windows x86_64 package",
    "Tag: $Tag",
    "Source commit: $npHead",
    "Source: https://github.com/johnstreetfighter/sf1-port/tree/$npHead",
    'Target: x86_64-pc-windows-msvc',
    'Profile: release; default features (tracker/libopenmpt included statically)',
    "Executable version: $npBanner",
    "Compiler: $npRust",
    "Packaged (UTC): $([DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ'))",
    "sf-game.exe SHA-256: $(Get-Sha256 (Join-Path $npPackage 'sf-game.exe'))",
    '',
    'Built from the tagged source. No ROMs, art/voice packs, settings or logs are included.',
    'This package does not grant new licenses; existing source/dependency notices are retained.'
)
[IO.File]::WriteAllLines((Join-Path $npPackage 'BUILD.txt'), $npBuildInfo)

$npQuickStart = @(
    "Street Fighter: The World Tournament - $Tag",
    '',
    '1. Extract the entire ZIP; keep shaders beside sf-game.exe.',
    '2. Use 64-bit Windows 10 or 11 with a Vulkan- or Direct3D 12-capable graphics driver.',
    '   If Windows reports a missing VCRUNTIME140.dll or MSVCP140.dll, install the latest Microsoft Visual C++ v14 Redistributable (x64):',
    '   https://aka.ms/vc14/vc_redist.x64.exe',
    '   Microsoft runtime information: https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist/',
    '3. Put your legally obtained US set 1 six-button sf ROM files in roms\sf beside the executable.',
    '   Keep the original ROM filenames. Alternatively run sf-game.exe --roms C:\path\to\sf.',
    '4. Run sf-game.exe. The ROM-missing screen explains missing or mismatched files.',
    '',
    'Optional packs go beside roms\sf as roms\sf1-port-art and roms\sf1-port-voice.',
    'These packs and the ROMs are separate from this download. SF1+ fighters need their approved art/player tables.',
    'Settings and sf-game.log are saved in %APPDATA%\sf1-port. Run sf-game.exe --console to see the log live.',
    'See README.md for controls and features.',
    '',
    'BUILD.txt records the exact source commit and executable checksum. licenses and THIRD_PARTY_NOTICES.md retain dependency notices.',
    'Verify the release SHA-256 against the downloaded archive.'
)
[IO.File]::WriteAllLines((Join-Path $npPackage 'QUICKSTART.txt'), $npQuickStart)

# Registry archives can use epoch timestamps, outside ZIP's date range.
# Adjust only staged metadata; every copied file's contents remain intact.
foreach ($npStagedFile in (Get-ChildItem -LiteralPath $npPackage -File -Recurse)) {
    if ($npStagedFile.LastWriteTime.Year -lt 1980) { $npStagedFile.LastWriteTime = [DateTime]'1980-01-01' }
}
Compress-Archive -LiteralPath $npPackage -DestinationPath $npZip -CompressionLevel Optimal
$npHash = Get-Sha256 $npZip
[IO.File]::WriteAllText("$npZip.sha256", "$npHash  $npName.zip`n")
Write-Output $npZip
Write-Output "$npZip.sha256"
