---
sidebar_position: 6
---

# How to start and debug Pyrsia Node using IDE

This tutorial describes how to start and debug Pyrsia using Microsoft Visual Code and IntelliJ IDEA.

## Prerequisites

Before continuing please make sure that you have cloned and compiled the Pyrsia sources. For more information please see [Developer Environment Setup](../get_involved/local_dev_setup.md). In this tutorial `PYRSIA_HOME` refers to the Pyrsia repository folder.

## IntelliJ IDEA Configuration

- Open the IDE and install [Rust plugin](https://www.jetbrains.com/rust/). If you use IDEA Ultimate make sure [Native Debugging Support plugin](https://plugins.jetbrains.com/plugin/12775-native-debugging-support) is installed.
- From the main menu select `File > Open`. Alternatively select `Open` in the `Welcome to IntelliJ IDEA` wizard and select the `PYRSIA_HOME` folder.
- When prompted to `Trust and Open Project 'pyrsia'?` select `Trust Project`.
- From the project configuration combo box select `Edit Configuration`; then in the Configuration window, select `+` or `Add New Run Configuration`.
- Select `Cargo` from the list of supported configurations.
- Rename the configuration to `Run Node`.
- In the `Command` field past the following:

`run --package pyrsia_node -- --pipeline-service-endpoint http://localhost:8080 --host 0.0.0.0 --port 7888 --listen /ip4/0.0.0.0/tcp/44002`

- Add the following vars to the `Environment Variables`:

```sh
DEV_MODE=on;
PYRSIA_ARTIFACT_PATH=/tmp/pyrsia/node;
RUST_LOG=pyrsia=debug,info;
```

- Point `Working Directory` to the `PYRSIA_HOME` folder.
- Confirm and save the configuration by pressing the `OK` button. In the event of `Error: No Rust toolchain specified` go to the `Settings/Preferences`, then `Languages & Frameworks > Rust`, then select `Downloads via Rustup`.
- Start the Pyrsia node by selecting `Run Node` from the configurations list and pressing the `Run` button next to it.
- **IDEA Ultimate only**. Start the debugging by selecting `Run Node` from the configurations list and pressing `Debug` button. The debugging related features (e.g. breakpoints) are only available when [Native Debugging Support plugin](https://plugins.jetbrains.com/plugin/12775-native-debugging-support) is installed.

## Microsoft Visual Code Configuration

- Open the IDE and install [rust-analyzer extension](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer), [CodeLLDB extension](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb) or [C/C++ extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode.cpptools)
- From the main menu select `File > Open Folder`.
- When asked `Do you trust the authors of the files in this folder?` select `Yes, I trust the authors...`.
- From the main menu select `Run > Add Configuration..`, then from the `Select Debugger` combo box select `LLDB`.
- When asked, `Cargo.toml was detected in this workspace...`, select `No`.
- In the newly created `launch.json` file, replace the generated configuration with the following:

```json
{
   "version": "0.2.0",
    "configurations": [
        {
            "type": "lldb",
            "request": "launch",
            "name": "Debug executable 'pyrsia_node'",
            "program": "${workspaceRoot}/target/debug/pyrsia_node",
            "args": [
                "-H", "0.0.0.0",
                "-p", "7888",
                "-L", "/ip4/0.0.0.0/tcp/44002"
            ],
            "env": {
                "DEV_MODE": "on",
                "RUST_LOG": "pyrsia=debug,info"
            },
            "cwd": "${workspaceRoot}"
        }
    ]
}
```

- Save the changes and start/debug the Pyrsia node by selecting `Run > Start Debugging` from the main menu.
