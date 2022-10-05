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

Function Install-NSSM {

    # check if nssm is installed
    $env:PATH = "${env:PATH};" + [System.Environment]::GetEnvironmentVariable('PATH','user')
    $testNSSM = (Get-Command nssm -ErrorAction SilentlyContinue).Path
    if ($testNSSM -eq $null) {
        # if nssm is not installed, check if scoop is installed
        $testScoop = (Get-Command scoop -ErrorAction SilentlyContinue).Path
        if ($testScoop -eq $null) {
            # if scoop is not installed, install it
            if ([Security.Principal.WindowsIdentity]::GetCurrent().Groups -contains 'S-1-5-32-544') {
                 Write-Host Installing scoop as admin... -foreground "green"
                 iex "& {$(irm get.scoop.sh)} -RunAsAdmin"
            } else {
                 Write-Host Installing scoop... -foreground "green"
                 iwr -useb get.scoop.sh | iex
            }
        }
        # install nssm
        Write-Host installing nssm... -foreground "green"
        scoop install nssm
    }

    # full path of nssm
    $NSSMPath = (Get-Item (Get-Command nssm).Path).DirectoryName
    Write-Host nssm found at $NSSMPath -foreground "green"

    # add path of scoop to a system custom env variable, to make it available to the next (elevated) script
    [Environment]::SetEnvironmentVariable("ScoopPath", $NSSMPath, "Machine")
}

Install-NSSM
