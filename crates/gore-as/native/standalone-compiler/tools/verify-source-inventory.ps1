param(
    [Parameter(Mandatory = $true)]
    [string] $UpstreamRoot
)

$ErrorActionPreference = 'Stop'
$componentRoot = Split-Path -Parent $PSScriptRoot
$inventoryPath = Join-Path $componentRoot 'SOURCE_INVENTORY.tsv'
$vendorRoot = Join-Path $componentRoot 'vendor\unreangel'
$rows = @(Import-Csv -LiteralPath $inventoryPath -Delimiter ([char]9))

function Get-CanonicalTextSha256 {
    param(
        [Parameter(Mandatory = $true)]
        [string] $LiteralPath
    )

    # Git may materialize the same text blob with LF or CRLF depending on the
    # checkout. Hash the repository-canonical form so the provenance check is
    # independent of that local setting while every other byte remains exact.
    $inputBytes = [System.IO.File]::ReadAllBytes($LiteralPath)
    $canonicalBytes = [byte[]]::new($inputBytes.Length)
    $writeIndex = 0
    for ($readIndex = 0; $readIndex -lt $inputBytes.Length; $readIndex++) {
        if (
            $inputBytes[$readIndex] -eq 13 -and
            $readIndex + 1 -lt $inputBytes.Length -and
            $inputBytes[$readIndex + 1] -eq 10
        ) {
            $canonicalBytes[$writeIndex] = 10
            $writeIndex++
            $readIndex++
        } else {
            $canonicalBytes[$writeIndex] = $inputBytes[$readIndex]
            $writeIndex++
        }
    }

    $sha256 = [System.Security.Cryptography.SHA256]::Create()
    try {
        $hash = $sha256.ComputeHash($canonicalBytes, 0, $writeIndex)
    } finally {
        $sha256.Dispose()
    }
    return -join ($hash | ForEach-Object { $_.ToString('x2') })
}

$revisions = @($rows.origin_revision | Sort-Object -Unique)
if ($revisions.Count -ne 1) {
    throw "Inventory must name exactly one upstream revision; found $($revisions.Count)."
}

$actualRevision = (& git -C $UpstreamRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $actualRevision -ne $revisions[0]) {
    throw "Upstream revision mismatch: expected $($revisions[0]), found $actualRevision."
}

foreach ($row in $rows | Where-Object { $_.match_kind -eq 'exact' -and $_.sha256 -ne '-' }) {
    $sourcePath = Join-Path $UpstreamRoot $row.source_path.Replace('/', '\')
    if (-not (Test-Path -LiteralPath $sourcePath -PathType Leaf)) {
        throw "Missing upstream inventory file: $($row.source_path)"
    }
    $sourceHash = Get-CanonicalTextSha256 -LiteralPath $sourcePath
    if ($sourceHash -ne $row.sha256) {
        throw "Upstream SHA-256 mismatch for $($row.source_path)."
    }

    if ($row.vendored_path -ne '-') {
        $vendorPath = Join-Path $componentRoot $row.vendored_path.Replace('/', '\')
        if (-not (Test-Path -LiteralPath $vendorPath -PathType Leaf)) {
            throw "Missing vendored inventory file: $($row.vendored_path)"
        }
        $vendorHash = Get-CanonicalTextSha256 -LiteralPath $vendorPath
        $expectedVendorHash = if ($row.disposition -eq 'vendored_modified') {
            if ([string]::IsNullOrWhiteSpace($row.vendored_sha256) -or $row.vendored_sha256 -eq '-') {
                throw "Modified vendored file has no downstream SHA-256: $($row.vendored_path)"
            }
            $row.vendored_sha256
        } else {
            if (-not [string]::IsNullOrWhiteSpace($row.vendored_sha256)) {
                throw "Byte-exact vendored file unexpectedly has a downstream SHA-256: $($row.vendored_path)"
            }
            $row.sha256
        }
        if ($vendorHash -ne $expectedVendorHash) {
            throw "Vendored file SHA-256 differs: $($row.vendored_path)"
        }
    }
}

$vendoredRows = @($rows | Where-Object { $_.vendored_path -ne '-' })
$listedVendorPaths = @($vendoredRows.vendored_path | Sort-Object -Unique)
$actualVendorPaths = @(
    Get-ChildItem -LiteralPath $vendorRoot -File -Recurse |
        ForEach-Object {
            'vendor/unreangel/' + $_.FullName.Substring($vendorRoot.Length + 1).Replace('\', '/')
        } |
        Sort-Object -Unique
)

if (Compare-Object -ReferenceObject $listedVendorPaths -DifferenceObject $actualVendorPaths) {
    throw 'Vendored tree and SOURCE_INVENTORY.tsv do not contain the same files.'
}

$sourceImports = @($rows | Where-Object { $_.disposition -in @('vendored_exact', 'vendored_modified') })
$sourceImportCount = $sourceImports.Count
$modifiedImportCount = @($sourceImports | Where-Object { $_.disposition -eq 'vendored_modified' }).Count
if ($sourceImportCount -ne 64 -or $modifiedImportCount -ne 2 -or $vendoredRows.Count -ne 65) {
    throw "Unexpected import count: $sourceImportCount source files, $($vendoredRows.Count) total vendored files."
}

Write-Host "Verified $sourceImportCount source imports ($modifiedImportCount downstream-modified) plus one license notice at $actualRevision."
