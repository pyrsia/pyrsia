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
+ Readme.txt

and also check the log file C:\tmp\log.txt if needed.

The pyrsia CLI is added to the system PATH. For instance, to test it run:

> pyrsia -s
Connected Peers Count:       1

The service folder contains the pyrsia_node executable that can be launched from any terminal.

For convenience, set these environment variables:

> set DEV_MODE=ON
> set RUST_LOG=pyrsia=debug

before running the file:

> pyrsia_node.exe
 2022-10-03T17:54:20.328Z DEBUG pyrsia_node > Parse CLI arguments
 2022-10-03T17:54:20.329Z DEBUG pyrsia_node > Create p2p components
 2022-10-03T17:54:20.336Z DEBUG pyrsia_node > Start p2p event loop
 2022-10-03T17:54:20.337Z DEBUG pyrsia_node > Create blockchain service component
...


Uninstalling Pyrsia
-------------------

To uninstall the Pyrsia software:

- Uninstall via Settings->Apps & features->Pyrsia, and press Uninstall

- Remove C:\Pyrsia folder (as it might contain some files created by the service)
