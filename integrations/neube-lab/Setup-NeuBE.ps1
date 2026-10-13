[CmdletBinding()]
param()
$ErrorActionPreference='Stop'
$taskSource='D:\DevData\Rainee\research\NeuBE'
$taskArchive='D:\DevData\Rainee\downloads\neube-models.zip'
$env:UV_CACHE_DIR='D:\DevData\Rainee\cache\uv'
$env:TEMP='D:\DevData\Rainee\temp';$env:TMP=$env:TEMP
if(-not(Test-Path -LiteralPath $taskSource)){git clone https://github.com/nv-tlabs/brushstroke_engine.git $taskSource;if($LASTEXITCODE){throw 'NeuBE clone failed'}}
if((git -C $taskSource rev-parse HEAD) -ne '093d3c7b047ef160314336c2034348c87f537a4b'){throw 'NeuBE source revision changed; review instead of overwriting'}
if(-not(Test-Path -LiteralPath $taskArchive)){Invoke-WebRequest -Uri 'https://nvidia-caat.s3.us-east-2.amazonaws.com/neube/models.zip' -OutFile $taskArchive}
if((Get-FileHash -LiteralPath $taskArchive).Hash -ne '6987F50E38300EE4B7C049EBE5865E73F272A6D04D2D4DA35354C3CDE5732133'){throw 'NeuBE archive hash changed'}
if(-not(Test-Path -LiteralPath (Join-Path $taskSource 'models'))){Expand-Archive -LiteralPath $taskArchive -DestinationPath $taskSource}
$taskEnv='D:\DevData\Rainee\venvs\neube'
if(-not(Test-Path -LiteralPath (Join-Path $taskEnv 'Scripts\python.exe'))){uv venv --python 'C:\Users\User\AppData\Local\Programs\Python\Python311\python.exe' $taskEnv;if($LASTEXITCODE){throw 'NeuBE venv failed'}}
$taskPython=Join-Path $taskEnv 'Scripts\python.exe'
uv pip install --python $taskPython torch==2.6.0 torchvision==0.21.0 --index-url https://download.pytorch.org/whl/cu124
if($LASTEXITCODE){throw 'CUDA PyTorch install failed'}
uv pip install --python $taskPython numpy==1.26.4 pillow==11.1.0 scipy==1.15.2 scikit-image==0.25.2 matplotlib==3.10.1 click==8.1.8 tqdm==4.67.1 requests==2.32.3 tensorboardX==2.6.2.2 psutil==7.0.0 ninja==1.11.1.3 easydict==1.13 setuptools==80.9.0
if($LASTEXITCODE){throw 'NeuBE runtime dependency install failed'}
Write-Output 'NeuBE research runtime ready on D. Local inference only; no training or portable redistribution.'
