@echo off
echo Running Pyrsia pre-commit validation.
echo This might take sometime, please do not interrupt if the screen is blank.

IF %1.==. GOTO NoClean
IF %1==clean echo Cleaning old build artifacts 
IF %1==clean cargo clean

:NoClean
cargo install cargo-audit
IF %ERRORLEVEL% NEQ 0 (ECHO Could not install cargo-audit.  &Exit /b 1)
cargo audit
IF %ERRORLEVEL% NEQ 0 (ECHO Cargo audit failed. &Exit /b 1)
cargo clippy
IF %ERRORLEVEL% NEQ 0 (ECHO Cargo clippy failed. &Exit /b 1)
rustup component add rustfmt
IF %ERRORLEVEL% NEQ 0 (ECHO Could not install rustfmt. &Exit /b 1)
cargo fmt --check
IF %ERRORLEVEL% NEQ 0 (ECHO Cargo format failed. &Exit /b 1)
cargo test --workspace
IF %ERRORLEVEL% NEQ 0 (ECHO Cargo test failed. &Exit /b 1)
cargo build --workspace
