param(
    [Parameter(Mandatory = $true)]
    [string] $UpstreamRoot
)

$ErrorActionPreference = 'Stop'
$componentRoot = Split-Path -Parent $PSScriptRoot
$inventoryPath = Join-Path $componentRoot 'SOURCE_INVENTORY.tsv'
$vendorRoot = Join-Path $componentRoot 'vendor\unreangel'
$rows = @(Import-Csv -LiteralPath $inventoryPath -Delimiter ([char]9))

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
    $sourceHash = (Get-FileHash -LiteralPath $sourcePath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($sourceHash -ne $row.sha256) {
        throw "Upstream SHA-256 mismatch for $($row.source_path)."
    }

    if ($row.vendored_path -ne '-') {
        $vendorPath = Join-Path $componentRoot $row.vendored_path.Replace('/', '\')
        if (-not (Test-Path -LiteralPath $vendorPath -PathType Leaf)) {
            throw "Missing vendored inventory file: $($row.vendored_path)"
        }
        $vendorHash = (Get-FileHash -LiteralPath $vendorPath -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($vendorHash -ne $row.sha256) {
            throw "Vendored file is not byte-exact: $($row.vendored_path)"
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

$sourceImportCount = @($rows | Where-Object { $_.disposition -eq 'vendored_exact' }).Count
if ($sourceImportCount -ne 64 -or $vendoredRows.Count -ne 65) {
    throw "Unexpected import count: $sourceImportCount source files, $($vendoredRows.Count) total vendored files."
}

Write-Host "Verified $sourceImportCount exact source imports plus one license notice at $actualRevision."
