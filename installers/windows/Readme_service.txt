Pyrsia installer for Windows
============================

How to install the MSI
----------------------

The MSI installer can be executed by double-clicking on it, but it is more convenient to install it from a command prompt, with non-admin rights.

- Press Win+R, type `cmd` and click OK to open the regular command prompt.

- To install under C:\Pyrsia, type:

> msiexec /i <path-to-msi>\pyrsia.msi ROOTDRIVE="C:"

- To install under C:\Pyrsia, and use a log file in C:\tmp, for instance, type:

> msiexec /i <path-to-msi>\pyrsia.msi /L*v C:\tmp\log.txt ROOTDRIVE="C:"

After the process ends, check the path C:\Pyrsia\Pyrsia. It should contain:

+ bin
   - pyrsia.exe
+ service
   - pyrsia_node.exe
   - InstallPyrsiaNodeAsWindowsService.ps1
   - InstallNSSM.ps1
+ Readme.txt

and also check the log file C:\tmp\log.txt if needed.

The pyrsia CLI is added to the system PATH. For instance, to test it run:

> pyrsia -s
Connected Peers Count:       1

The service is installed and started. You can check it running:

> nssm status PyrsiaService
SERVICE_RUNNING

If everything works as expected, the files under C:\Pyrsia\Pyrsia should be:

+ bin
   - pyrsia.exe
+ service
   - pyrsia_node.exe
   - InstallPyrsiaNodeAsWindowsService.ps1
   - InstallNSSM.ps1
   - pyrsia_logs.txt
   + pyrsia
      -  p2p_keypair.ser
+ Readme.txt

Note: scoop [https://scoop.sh/] and nssm [https://nssm.cc/] packages are installed during the process.
NSSM is required to create and run the PyrsiaService service, so it can not be removed once the MSI is installed.


Uninstalling Pyrsia
-------------------

To uninstall the Pyrsia software:

- Press Win+R, type `cmd` and press Ctrl+Shift+Enter to open a command prompt with admin rights.

- Type:
> nssm stop PyrsiaService
> nssm remove PyrsiaService confirm

- Uninstall via Settings->Apps & features->Pyrsia, and press Uninstall

- Remove C:\Pyrsia folder (as it still contains some files created by the service)

- Uninstall NSSM, if needed:

> scoop uninstall nssm

- Uninstall Scoop, if needed:

> scoop uninstall scoop

