param(
    [Parameter(Mandatory = $true)]
    [string]$HeaderPath,

    [Parameter(Mandatory = $true)]
    [string]$ScriptCache,

    [string]$GoreExe = (Join-Path $PSScriptRoot '..\target\debug\gore.exe'),

    [string]$Output = (Join-Path $PSScriptRoot '..\apps\save-editor\assets\glossary_npc_catalog.json'),

    [string]$TextOutput = (Join-Path $PSScriptRoot '..\apps\save-editor\assets\glossary_segment_text_catalog.json')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

foreach ($required in @($HeaderPath, $ScriptCache, $GoreExe)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
        throw "Required file does not exist: $required"
    }
}

$documentPattern = '^class UDocument_Glossary_(?<id>\w+) : public UGlossary(?<camp>OldCamp|NewCamp|SwampCamp|Outsiders)Document'
$segmentPattern = '^class UDocumentSegment_Glossary_(?<id>\w+) : public UDocumentSegment'
$headerLines = Get-Content -LiteralPath $HeaderPath

$documents = @(
    foreach ($line in $headerLines) {
        if ($line -match $documentPattern) {
            [pscustomobject]@{
                id = $Matches.id
                camp = switch ($Matches.camp) {
                    'OldCamp' { 'oldCamp' }
                    'NewCamp' { 'newCamp' }
                    'SwampCamp' { 'swampCamp' }
                    'Outsiders' { 'outsiders' }
                }
            }
        }
    }
)

$segmentNames = @(
    foreach ($line in $headerLines) {
        if ($line -match $segmentPattern) {
            $Matches.id
        }
    }
)

if ($documents.Count -eq 0 -or $segmentNames.Count -eq 0) {
    throw "No NPC glossary documents/segments found in $HeaderPath"
}

# `GlossaryForCharacter` is the canonical save UniqueName. It is more precise
# than casing/prefix inference from CharacterDefinition classes (Caine, Gorn,
# orcs, and Sleeper variants are otherwise ambiguous).
$metadataByDocument = @{}
$currentDocument = $null
$oldErrorActionPreference = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
& $GoreExe as decompile $ScriptCache Document_Glossary_ --max 200000 2>$null |
    ForEach-Object {
        if ($_ -match '^// .*\.(UDocument_Glossary_[A-Za-z0-9_]+)::__InitDefaults$') {
            $currentDocument = $Matches[1]
        } elseif (
            $null -ne $currentDocument -and
            $_ -match 'Metadata\.Add\(n"GlossaryForCharacter",\s*"([^"]+)"\)'
        ) {
            $metadataByDocument[$currentDocument] = $Matches[1]
        }
    }
$decompileExitCode = $LASTEXITCODE
$ErrorActionPreference = $oldErrorActionPreference
if ($decompileExitCode -ne 0) {
    throw "gore as decompile failed with exit code $decompileExitCode"
}

# A save stores only segment class ids such as `...Wolf_Entry2`. The actual
# player-facing paragraphs are LocText references in each segment's
# BuildSegment implementation. Keep only those stable ids in the app asset;
# the already-extracted game localization catalog resolves them at runtime.
$textIdsBySegment = @{}
$currentSegment = $null
$oldErrorActionPreference = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
& $GoreExe as emit $ScriptCache Document_Glossary_ --max 200000 2>$null |
    ForEach-Object {
        if ($_ -match '^class ') {
            $currentSegment = $null
            if ($_ -match '^class UDocumentSegment_Glossary_(?<id>[A-Za-z0-9_]+) : UDocumentSegment') {
                $currentSegment = $Matches.id
                $textIdsBySegment[$currentSegment] = [System.Collections.Generic.List[string]]::new()
            }
        } elseif (
            $null -ne $currentSegment -and
            $_ -match 'LocText\("(?<id>[^"]+)"\)'
        ) {
            $textIdsBySegment[$currentSegment].Add($Matches.id)
        }
    }
$emitExitCode = $LASTEXITCODE
$ErrorActionPreference = $oldErrorActionPreference
if ($emitExitCode -ne 0) {
    throw "gore as emit failed with exit code $emitExitCode"
}

$missingTextSegments = @(
    $segmentNames | Where-Object {
        -not $textIdsBySegment.ContainsKey($_) -or $textIdsBySegment[$_].Count -eq 0
    }
)
if ($missingTextSegments.Count -ne 0) {
    throw "Missing LocText references for $($missingTextSegments.Count) glossary segments: $($missingTextSegments -join ', ')"
}

$unexpectedTextSegments = @(
    $textIdsBySegment.Keys | Where-Object { $_ -notin $segmentNames }
)
if ($unexpectedTextSegments.Count -ne 0) {
    throw "Emitted unknown glossary segments: $($unexpectedTextSegments -join ', ')"
}

$textReferenceCount = ($textIdsBySegment.Values | ForEach-Object Count | Measure-Object -Sum).Sum
if ($segmentNames.Count -ne 734 -or $textReferenceCount -ne 759) {
    throw "Unexpected glossary text coverage: $($segmentNames.Count) segments / $textReferenceCount references (expected 734 / 759)"
}

function ConvertTo-SegmentLabel([string]$value) {
    $label = $value -replace '_', ' '
    $label = $label -creplace '(?<=[A-Z])(?=[A-Z][a-z])', ' '
    $label = $label -creplace '(?<=[a-z0-9])(?=[A-Z])', ' '
    return $label
}

function Get-SegmentRoles([string]$segmentId) {
    $roles = [System.Collections.Generic.List[string]]::new()
    if ($segmentId -match '(^|_)Introduction(?:_|$)') { $roles.Add('portrait') }
    if ($segmentId -match '(^|_)(Trader|Dealer)(?:_|$)') { $roles.Add('trader') }
    if ($segmentId -match '(^|_)(Teach|Train)') { $roles.Add('teacher') }
    if ($segmentId -match '(^|_)Armor') { $roles.Add('armorer') }
    if ($segmentId -match '(^|_)Dead(?:_|$)') { $roles.Add('dead') }
    if ($segmentId -match '(^|_)(Hostile|Enemy|Angry)(?:_|$)') { $roles.Add('hostile') }
    return @($roles)
}

$idsLongestFirst = @($documents.id | Sort-Object { $_.Length } -Descending)
$segmentsByDocument = @{}
foreach ($document in $documents) {
    $segmentsByDocument[$document.id] = [System.Collections.Generic.List[object]]::new()
}

foreach ($fullSegmentId in $segmentNames) {
    $documentId = $idsLongestFirst |
        Where-Object { $fullSegmentId.StartsWith("${_}_", [StringComparison]::Ordinal) } |
        Select-Object -First 1
    if ($null -eq $documentId) {
        continue
    }
    $segmentId = $fullSegmentId.Substring($documentId.Length + 1)
    $roles = @(Get-SegmentRoles $segmentId)
    $segmentsByDocument[$documentId].Add([ordered]@{
        id = $segmentId
        class = "/Script/Angelscript.DocumentSegment_Glossary_$fullSegmentId"
        label = ConvertTo-SegmentLabel $segmentId
        roles = $roles
    })
}

$catalog = @(
    foreach ($document in ($documents | Sort-Object camp, id)) {
        $metadataKey = "UDocument_Glossary_$($document.id)"
        if (-not $metadataByDocument.ContainsKey($metadataKey)) {
            throw "Missing GlossaryForCharacter metadata for $metadataKey"
        }
        $segments = @($segmentsByDocument[$document.id] | Sort-Object id)
        if ($segments.Count -eq 0) {
            throw "No segment classes found for $metadataKey"
        }
        [ordered]@{
            id = $document.id
            uniqueName = $metadataByDocument[$metadataKey]
            documentClass = "/Script/Angelscript.Document_Glossary_$($document.id)"
            camp = $document.camp
            segments = $segments
        }
    }
)

$expectedCampCounts = [ordered]@{
    oldCamp = 63
    newCamp = 41
    swampCamp = 34
    outsiders = 22
}
foreach ($camp in $expectedCampCounts.Keys) {
    $actual = @($catalog | Where-Object camp -eq $camp).Count
    if ($actual -ne $expectedCampCounts[$camp]) {
        throw "Unexpected $camp document count: $actual (expected $($expectedCampCounts[$camp]))"
    }
}

$outputDirectory = Split-Path -Parent $Output
if ($outputDirectory) {
    New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null
}
$json = $catalog | ConvertTo-Json -Depth 8
[IO.File]::WriteAllText(
    [IO.Path]::GetFullPath($Output),
    $json,
    [Text.UTF8Encoding]::new($false)
)
Write-Host "Wrote $($catalog.Count) NPC glossary documents to $Output"

$textCatalog = [ordered]@{}
foreach ($segmentName in ($segmentNames | Sort-Object)) {
    $textCatalog["/Script/Angelscript.DocumentSegment_Glossary_$segmentName"] = @(
        $textIdsBySegment[$segmentName]
    )
}
$textOutputDirectory = Split-Path -Parent $TextOutput
if ($textOutputDirectory) {
    New-Item -ItemType Directory -Path $textOutputDirectory -Force | Out-Null
}
$textJson = $textCatalog | ConvertTo-Json -Depth 4
[IO.File]::WriteAllText(
    [IO.Path]::GetFullPath($TextOutput),
    $textJson,
    [Text.UTF8Encoding]::new($false)
)
Write-Host "Wrote $($textCatalog.Count) glossary segment text mappings ($textReferenceCount references) to $TextOutput"
