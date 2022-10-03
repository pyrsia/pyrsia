#   Copyright 2021 JFrog Ltd
#
#   Licensed under the Apache License, Version 2.0 (the "License");
#   you may not use this file except in compliance with the License.
#   You may obtain a copy of the License at
#
#       http://www.apache.org/licenses/LICENSE-2.0
#
#   Unless required by applicable law or agreed to in writing, software
#   distributed under the License is distributed on an "AS IS" BASIS,
#   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#   See the License for the specific language governing permissions and
#   limitations under the License.

Function Install-Service {
     param(
        [Parameter(Mandatory=$true)][string]$serviceName ,
        [Parameter(Mandatory=$true)][string]$serviceExecutable ,
        [Parameter(Mandatory=$false)][string]$serviceExecutableArgs ,
        [Parameter(Mandatory=$false)][string]$serviceAppDirectory,
        [Parameter(Mandatory=$false)][string]$serviceDescription,
        [Parameter(Mandatory=$false)][string[]]$serviceAppEnvironmentExtra,
        [Parameter(Mandatory=$false)][string]$serviceAppStdout,
        [Parameter(Mandatory=$false)][string]$serviceAppStderr
    )

    $env:PATH = [System.Environment]::GetEnvironmentVariable('PATH','user') + ";" + [System.Environment]::GetEnvironmentVariable('PATH','system')
    $testNSSM = (Get-Command nssm -ErrorAction SilentlyContinue).Path
    if ($testNSSM -eq $null) {
        # Error: this shouldn't happen, but for now don't install the service and
        # let the installer continue
        Write-Host "Error: nssm not found in path: ${env:PATH}" -foreground "red"
        exit 0
    }
    # directory where nssm should be already installed
    $NSSMPath = (Get-Item (Get-Command nssm).Path).DirectoryName

    Write-Host Installing service $serviceName -foreground "green"
    Write-Host "NSSM path:" $NSSMPath
    Write-Host "Service name:" $serviceName
    Write-Host "Service executable:" $serviceExecutable
    Write-Host "Service executable args:" $serviceExecutableArgs
    Write-Host "Service app directory:" $serviceAppDirectory
    Write-Host "Service description:" $serviceDescription
    Write-Host "Service environment:" $serviceAppEnvironmentExtra
    Write-Host "Service sdtout:" $serviceAppStdout
    Write-Host "Service stderr:" $serviceAppStderr

    push-location
    Set-Location $NSSMPath

    # Check for an existing service
    $service = Get-Service $serviceName -ErrorAction SilentlyContinue
    if ($service)
    {
        # stop and remove previous service
        Write-host service $service.Name is $service.Status
        Write-Host Removing $serviceName service
        &.\nssm.exe stop $serviceName
        &.\nssm.exe remove $serviceName confirm
    }

    # install service
    Write-Host Installing $serviceName as a service
    &.\nssm.exe install $serviceName $serviceExecutable $serviceExecutableArgs

    # set app directory
    if ($serviceAppDirectory)
    {
        Write-host setting app directory to $serviceAppDirectory -foreground "green"
        &.\nssm.exe set $serviceName AppDirectory $serviceAppDirectory
    }

    # set app description
    if ($serviceDescription)
    {
        Write-host setting app description to $serviceDescription -foreground "green"
        &.\nssm.exe set $serviceName DESCRIPTION $serviceDescription
    }

    # setting environment variables
    if ($serviceAppEnvironmentExtra)
    {
        Write-host setting app env to $serviceAppEnvironmentExtra -foreground "green"
        &.\nssm.exe set $serviceName AppEnvironmentExtra $serviceAppEnvironmentExtra
    }

    # set app stdout logs
    if ($serviceAppStdout)
    {
        Write-host setting app stdout logs to $serviceAppStdout -foreground "green"
        &.\nssm.exe set $serviceName AppStdout $serviceAppStdout
    }

    # set app stderr logs
    if ($serviceAppStderr)
    {
        Write-host setting app stderr logs to $serviceAppStderr -foreground "green"
        &.\nssm.exe set $serviceName AppStderr $serviceAppStderr
    }

    # start service
    &.\nssm.exe start $serviceName
    pop-location
}

# store current location, and cd into ps script root
$prevPwd = $PWD
Set-Location -ErrorAction Stop -LiteralPath $PSScriptRoot

try {
    $ServiceName = "PyrsiaService"
    $BinaryPath = "${PWD}\pyrsia_node.exe"
    $serviceAppDirectory = $PWD
    $Description = "Pyrsia: the distributed package manager service"
    $serviceAppEnvironmentExtra = "RUST_LOG=pyrsia=debug", "DEV_MODE=on"
    $serviceAppStdout = "${PWD}\pyrsia_logs.txt"
    $serviceAppStderr = "${PWD}\pyrsia_logs.txt"

    # Call function to create and install the service
    Install-Service -ServiceName $ServiceName -serviceExecutable $BinaryPath -serviceAppDirectory $serviceAppDirectory -serviceDescription $Description -serviceAppEnvironmentExtra $serviceAppEnvironmentExtra -serviceAppStdout $serviceAppStdout -serviceAppStderr $serviceAppStderr
} finally {
    # restore initial location
    $prevPwd | Set-Location
}
