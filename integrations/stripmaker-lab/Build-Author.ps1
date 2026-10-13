[CmdletBinding()]
param([string]$SourceRoot='D:\DevData\Rainee\research\StripMaker')
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot '..\storage\Use-DDrive.ps1') -Target Editor
$revision='810a61a03c2177a61f8e32b8ce72c33dccfac49f'
$head=& git -C $SourceRoot rev-parse HEAD
if($LASTEXITCODE -or $head -ne $revision){throw 'Expected the pinned author checkout. No source reset or checkout is performed by this script.'}
$changes=& git -C $SourceRoot status --porcelain
if($LASTEXITCODE -or $changes){throw 'Preserve author source changes; use a clean pinned research checkout for this test.'}
$sdkRoot='D:\DevData\Rainee\tools\gurobi-sdk-13.0.3\gurobi1303\win64'
$researchBuild='D:\DevData\Rainee\build\stripmaker-author'
$depCache='D:\DevData\Rainee\cache\stripmaker-cmake'
$spdSource=Join-Path $depCache 'spdlog-src'
$spdRevision='cf6f1dd01e660d5865d68bf5fa78f6376b89470a'
$python='D:\DevData\Rainee\venvs\gurobi-evaluation\Scripts\python.exe'
if(-not(Test-Path -LiteralPath (Join-Path $sdkRoot 'lib\gurobi130.lib'))){throw 'Run Setup-GurobiSdk.ps1 first.'}
if(-not(Test-Path -LiteralPath $python)){throw 'Create the D-drive research Python environment first; see README.'}
& $python -c 'import distutils.sysconfig'
if($LASTEXITCODE){throw 'Legacy pybind11 configuration requires distutils. Install setuptools==80.9.0 in this research environment.'}
if(-not(Test-Path -LiteralPath $spdSource)){
    New-Item -ItemType Directory -Path $depCache -Force | Out-Null
    & git clone --depth 1 --branch v1.5.0 https://github.com/gabime/spdlog.git $spdSource
    if($LASTEXITCODE){throw 'spdlog download failed.'}
}
$spdHead=& git -C $spdSource rev-parse HEAD
if($LASTEXITCODE -or $spdHead -ne $spdRevision){throw 'Unexpected spdlog reference revision.'}
# SketchConnectivity calls find_package(spdlog), despite the parent's existing target.
# Install its matching package config into an isolated D-drive prefix first.
$spdBuild='D:\DevData\Rainee\build\stripmaker-spdlog'
$prefix='D:\DevData\Rainee\tools\stripmaker-deps'
$cmSdk=$sdkRoot.Replace('\','/')
$cmCache=$depCache.Replace('\','/')
$cmPython=$python.Replace('\','/')
$cmPrefix=$prefix.Replace('\','/')
$flags='/EHsc /D_SILENCE_STDEXT_ARR_ITERS_DEPRECATION_WARNING'
& cmake -S $spdSource -B $spdBuild -G 'Visual Studio 17 2022' -A x64 '-DCMAKE_POLICY_VERSION_MINIMUM=3.5' ("-DCMAKE_INSTALL_PREFIX=$prefix") ("-DCMAKE_CXX_FLAGS=$flags") '-DSPDLOG_BUILD_EXAMPLE=OFF' '-DSPDLOG_BUILD_TESTS=OFF' '-DSPDLOG_INSTALL=ON'
if($LASTEXITCODE){throw 'spdlog configure failed.'}
& cmake --build $spdBuild --config Release --parallel 4 -- /verbosity:quiet
if($LASTEXITCODE){throw 'spdlog build failed.'}
& cmake --install $spdBuild --config Release
if($LASTEXITCODE){throw 'spdlog local install failed.'}
# Resolve v13 explicitly without changing the author's FindGUROBI.cmake.
# Disable SketchConnectivity's optional Gurobi feature definition only;
# StripMaker/StrokeStrip optimization still links Gurobi and needs a license.
& cmake -S $SourceRoot -B $researchBuild -G 'Visual Studio 17 2022' -A x64 '-DCMAKE_POLICY_VERSION_MINIMUM=3.5' ("-DFETCHCONTENT_BASE_DIR=$cmCache") ("-DGUROBI_DIR=$cmSdk") ("-DGUROBI_INCLUDE_DIRS=$cmSdk/include") ("-DGUROBI_LIBRARY=$cmSdk/lib/gurobi130.lib") ("-DGUROBI_CXX_LIBRARY=$cmSdk/lib/gurobi_c++md2017.lib") ("-DGUROBI_CXX_DEBUG_LIBRARY=$cmSdk/lib/gurobi_c++mdd2017.lib") ("-DPYTHON_EXECUTABLE=$cmPython") ("-Dspdlog_DIR=$cmPrefix/lib/cmake/spdlog") '-DCMAKE_INSTALL_PREFIX=D:/DevData/Rainee/tools/stripmaker-author' ("-DCMAKE_CXX_FLAGS=$flags") '-DWITH_GUROBI=OFF' '-DEIGEN_BUILD_TESTING=OFF' '-DBUILD_TESTING=OFF' '-DEIGEN_BUILD_DOC=OFF'
if($LASTEXITCODE){throw 'StripMaker configure failed.'}
# The paper's Python 3.8 bindings need their own compatibility work on Python 3.13.
# Do not build or claim those bindings here. These are the native reference CLIs.
& cmake --build $researchBuild --config Release --target clustering-solve clustering-endend strokestrip --parallel 4 -- /verbosity:quiet
if($LASTEXITCODE){throw 'Author reference CLI build failed.'}
[pscustomobject]@{SourceRevision=$head;ReferenceBinaries=(Join-Path $researchBuild 'Release');PythonBindings='not built';LicenseGate='Test-GurobiSdk.ps1 must pass before processing drawings'} | ConvertTo-Json
