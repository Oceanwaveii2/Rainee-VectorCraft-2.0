[CmdletBinding()]
param([string]$SourceRoot='D:\DevData\Rainee\research\StripMaker')
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot '..\storage\Use-DDrive.ps1') -Target Editor
$head=& git -C $SourceRoot rev-parse HEAD
if($LASTEXITCODE -or $head -ne '810a61a03c2177a61f8e32b8ce72c33dccfac49f'){throw 'Expected pinned author revision.'}
$changes=& git -C $SourceRoot status --porcelain
if($LASTEXITCODE -or $changes){throw 'Author checkout must stay clean.'}
$sdk='D:\DevData\Rainee\tools\scip-10.1.0\scipoptsuite-10.1.0-win-x64'
$download='D:\DevData\Rainee\downloads\scip\scipoptsuite-10.1.0-win-x64.zip'
$expected='E71BAFB8CE3C26386AE0D93CDD51867F21EB74B70986BBD696B7B864A3C355FF'
if(-not(Test-Path -LiteralPath $download)){
  New-Item -ItemType Directory -Force (Split-Path $download) | Out-Null
  Invoke-WebRequest 'https://github.com/scipopt/scip/releases/download/v10.1.0/scipoptsuite-10.1.0-win-x64.zip' -OutFile $download
}
if((Get-FileHash -LiteralPath $download).Hash -ne $expected){throw 'SCIP archive checksum mismatch.'}
if(-not(Test-Path -LiteralPath "$sdk\lib\libscip.lib")){
  Expand-Archive -LiteralPath $download -DestinationPath (Split-Path $sdk)
}
$build='D:\DevData\Rainee\build\stripmaker-scip'
$cache='D:/DevData/Rainee/cache/stripmaker-cmake'
$osqpSource='D:\DevData\Rainee\research\osqp-1.0.0'
if(-not(Test-Path -LiteralPath "$osqpSource\CMakeLists.txt")){
  & git clone --depth 1 --branch v1.0.0 https://github.com/osqp/osqp.git $osqpSource
  if($LASTEXITCODE){throw 'OSQP download failed.'}
}
$osqpHead=& git -C $osqpSource rev-parse HEAD
$osqpChanges=& git -C $osqpSource status --porcelain
if($LASTEXITCODE -or $osqpHead -ne '236713ce9a56c182ac3230d52108f952afce1523' -or $osqpChanges){throw 'Expected clean pinned OSQP source.'}
$osqpBuild='D:\DevData\Rainee\build\osqp-1.0.0'
$osqpPrefix='D:/DevData/Rainee/tools/osqp-msvc-1.0.0'
& cmake -S $osqpSource -B $osqpBuild -G 'Visual Studio 17 2022' -A x64 "-DCMAKE_INSTALL_PREFIX=$osqpPrefix" `
  "-DFETCHCONTENT_BASE_DIR=$cache/osqp" '-DOSQP_ALGEBRA_BACKEND=builtin' '-DOSQP_USE_LONG=OFF' '-DOSQP_USE_FLOAT=OFF' `
  '-DOSQP_BUILD_SHARED_LIB=OFF' '-DOSQP_BUILD_STATIC_LIB=ON' '-DOSQP_BUILD_DEMO_EXE=OFF' '-DOSQP_BUILD_UNITTESTS=OFF' `
  '-DOSQP_CODEGEN=OFF' '-DOSQP_ENABLE_DERIVATIVES=OFF' '-DOSQP_ENABLE_INTERRUPT=OFF'
if($LASTEXITCODE){throw 'OSQP configure failed.'}
$qdldlSource='D:\DevData\Rainee\cache\stripmaker-cmake\osqp\qdldl-src'
$qdldlHead=& git -C $qdldlSource rev-parse HEAD
$qdldlChanges=& git -C $qdldlSource status --porcelain
if($LASTEXITCODE -or $qdldlHead -ne '138fdac58b9cd1c4137ff1b99152c8108a6cff5b' -or $qdldlChanges){throw 'Expected clean pinned QDLDL v0.1.8.'}
& cmake --build $osqpBuild --config Release --parallel 4 -- /verbosity:quiet
if($LASTEXITCODE){throw 'OSQP build failed.'}
& cmake --install $osqpBuild --config Release
if($LASTEXITCODE){throw 'OSQP install failed.'}
$python='D:/DevData/Rainee/venvs/stripmaker-build/Scripts/python.exe'
$uv='D:\DevData\Rust\cargo\bin\uv.exe'
$env:UV_CACHE_DIR='D:\DevData\Rainee\cache\uv'
if(-not(Test-Path -LiteralPath $python)){
  & $uv venv --python 'C:\Users\User\AppData\Local\Programs\Python\Python313\python.exe' 'D:\DevData\Rainee\venvs\stripmaker-build'
  if($LASTEXITCODE){throw 'Build bootstrap environment failed.'}
  & $uv pip install --python $python 'setuptools==80.9.0'
  if($LASTEXITCODE){throw 'Legacy pybind11 configure dependency failed.'}
}
$prefix='D:/DevData/Rainee/tools/stripmaker-deps'
if(-not(Test-Path -LiteralPath "$prefix/lib/cmake/spdlog/spdlogConfig.cmake")){
  $spdSource='D:\DevData\Rainee\cache\stripmaker-cmake\spdlog-src'
  if(-not(Test-Path -LiteralPath $spdSource)){
    & git clone --depth 1 --branch v1.5.0 https://github.com/gabime/spdlog.git $spdSource
    if($LASTEXITCODE){throw 'spdlog download failed.'}
  }
  $spdHead=& git -C $spdSource rev-parse HEAD
  if($LASTEXITCODE -or $spdHead -ne 'cf6f1dd01e660d5865d68bf5fa78f6376b89470a'){throw 'Unexpected spdlog revision.'}
  $spdBuild='D:\DevData\Rainee\build\stripmaker-spdlog'
  & cmake -S $spdSource -B $spdBuild -G 'Visual Studio 17 2022' -A x64 '-DCMAKE_POLICY_VERSION_MINIMUM=3.5' "-DCMAKE_INSTALL_PREFIX=$prefix" '-DCMAKE_CXX_FLAGS=/EHsc /D_SILENCE_STDEXT_ARR_ITERS_DEPRECATION_WARNING' '-DSPDLOG_BUILD_EXAMPLE=OFF' '-DSPDLOG_BUILD_TESTS=OFF' '-DSPDLOG_INSTALL=ON'
  if($LASTEXITCODE){throw 'spdlog configure failed.'}
  & cmake --build $spdBuild --config Release --parallel 4 -- /verbosity:quiet
  if($LASTEXITCODE){throw 'spdlog build failed.'}
  & cmake --install $spdBuild --config Release
  if($LASTEXITCODE){throw 'spdlog local install failed.'}
}
# Only this PowerShell process sees the solver DLLs. Leave the editor/global PATH alone.
$priorPath=$env:PATH
try {
$env:PATH="$sdk\bin;$env:PATH"
& cmake -S (Join-Path $PSScriptRoot 'scip') -B $build -G 'Visual Studio 17 2022' -A x64 `
  '-DCMAKE_POLICY_VERSION_MINIMUM=3.5' "-DSTRIPMAKER_SOURCE=$($SourceRoot.Replace('\','/'))" "-DSCIP_SDK=$($sdk.Replace('\','/'))" `
  "-DOSQP_PREFIX=$osqpPrefix" `
  '-DCMAKE_INSTALL_PREFIX=D:/DevData/Rainee/tools/stripmaker-scip' `
  "-DFETCHCONTENT_BASE_DIR=$cache" "-DPYTHON_EXECUTABLE=$python" "-Dspdlog_DIR=$prefix/lib/cmake/spdlog" `
  '-DCMAKE_CXX_FLAGS=/EHsc /D_SILENCE_STDEXT_ARR_ITERS_DEPRECATION_WARNING' `
  '-DWITH_GUROBI=OFF' '-DEIGEN_BUILD_TESTING=OFF' '-DBUILD_TESTING=OFF' '-DEIGEN_BUILD_DOC=OFF'
if($LASTEXITCODE){throw 'SCIP research configure failed.'}
& cmake --build $build --config Release --target scip-adapter-tests scip-pipeline --parallel 4 -- /verbosity:quiet
if($LASTEXITCODE){throw 'SCIP research build failed.'}
& "$build\Release\scip-adapter-tests.exe"
if($LASTEXITCODE){throw 'SCIP adapter tests failed.'}
[pscustomobject]@{SourceRevision=$head;Solver='SCIP 10.1.0 + OSQP 1.0.0 (CPU)';ArchiveSHA256=$expected;Binaries="$build\author\Release";EditorIntegration='not yet connected'} | ConvertTo-Json
} finally { $env:PATH=$priorPath }
