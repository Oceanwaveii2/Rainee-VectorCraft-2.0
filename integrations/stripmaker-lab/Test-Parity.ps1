[CmdletBinding()]
param([string]$SourceRoot='D:\DevData\Rainee\research\StripMaker')
$ErrorActionPreference='Stop'
$taskRevision=& git -C $SourceRoot rev-parse HEAD
if($LASTEXITCODE -or $taskRevision -ne '810a61a03c2177a61f8e32b8ce72c33dccfac49f'){throw 'Author oracle requires the pinned source revision.'}
$taskChanges=& git -C $SourceRoot status --porcelain
if($LASTEXITCODE -or $taskChanges){throw 'Author oracle requires a clean research checkout, including its headers.'}
. (Join-Path $PSScriptRoot '..\storage\Use-DDrive.ps1') -Target Editor
$env:CARGO_TARGET_DIR='D:\DevData\Rainee\build\stripmaker-classifier-lab'
$taskBuild=$env:CARGO_TARGET_DIR
New-Item -ItemType Directory -Path $taskBuild -Force | Out-Null
Push-Location $PSScriptRoot
try {
    cargo test --locked
    if($LASTEXITCODE){throw 'Classifier unit tests failed.'}
    cargo build --locked
    if($LASTEXITCODE){throw 'Classifier build failed.'}
    $taskExe=Join-Path $taskBuild 'debug\stripmaker-classifier-lab.exe'
    # Verify model hashes through the Rust parser before compiling author C++.
    $taskRows=@()
    foreach($taskModel in @('local','contextual')) {
        $taskJson=@(& $taskExe $SourceRoot $taskModel --probes)
        if($LASTEXITCODE){throw 'Pinned model validation/probe generation failed.'}
        foreach($taskLine in $taskJson){$taskRows+=($taskLine|ConvertFrom-Json)}
    }
    $taskVsWhere='C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
    $taskVs= & $taskVsWhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if(-not $taskVs){throw 'MSVC C++ tools needed for independent author-code parity test.'}
    $taskSdkScript=Join-Path $taskVs 'Common7\Tools\Launch-VsDevShell.ps1'
    & $taskSdkScript -Arch amd64 -HostArch amd64 -SkipAutomaticLocation
    Push-Location $taskBuild
    try {
        & cl.exe /nologo /EHsc /O2 (Join-Path $PSScriptRoot 'reference.cpp') (Join-Path $SourceRoot 'src\classifier\forest.cpp') (Join-Path $SourceRoot 'src\classifier\forest_sec.cpp') '/Fe:forest-reference.exe'
        if($LASTEXITCODE){throw 'Author forest oracle build failed.'}
        $taskCulture=[Globalization.CultureInfo]::InvariantCulture
        $taskInput=$taskRows | ForEach-Object { "$($_.features.Count) " + (($_.features | ForEach-Object {([double]$_).ToString('R',$taskCulture)}) -join ' ') }
        $taskPredictions=@($taskInput | & (Join-Path $taskBuild 'forest-reference.exe'))
        if($LASTEXITCODE -or $taskPredictions.Count -ne $taskRows.Count){throw 'Author oracle returned an incomplete result.'}
        $taskMax=0.0
        for($taskI=0;$taskI -lt $taskRows.Count;$taskI++) {
            $taskExpected=[double]::Parse($taskPredictions[$taskI],$taskCulture)
            $taskMax=[Math]::Max($taskMax,[Math]::Abs($taskExpected-[double]$taskRows[$taskI].probability))
        }
        if($taskMax -gt 1e-14){throw "Inference differs from author code: $taskMax"}
        [pscustomobject]@{Models=2;Trees=300;FeatureRows=$taskRows.Count;MaxAbsoluteError=$taskMax;Scope='inference parity, not feature-extraction or sketch-quality acceptance'} | ConvertTo-Json
    } finally {Pop-Location}
} finally {Pop-Location}
