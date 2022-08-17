# Submit a PR

If you haven't already, read about what we consider as [Good PRs](./good_pr.md).

When developing new features for Pyrsia, we aim for the best quality code possible. Here's the steps with some "How To"s on getting there.

We also have "pre commit" scripts (located in the root of the repository) which will run on of these steps.
This is a easy way to prepare your changes ahead of opening a pull request.

## Builds and Tests Pass

The base line is making sure all the code compiles and every test passes.

❗ This is enforced by our Action jobs.

```sh
cargo build --all-targets
cargo test --workspace
```

### A release build

Pyrsia provides a release build that is installed through system package managers. This should be built and run as part of the integration tests.

ℹ️ For major changes, this is recommended.

```sh
cargo build --all-targets --release
```

## Format and Linting

> ⚠️ _Make sure to follow the install instructions [here](#install-linters) on your first time_

We have dedicated ourselves to the community and following the standard practices such as <https://github.com/rust-dev-tools/fmt-rfcs>.

❗ We are strict about letting warnings into our code base. This is partially enforced by our Action jobs.

```sh
cargo fmt
cargo clippy
```

### Install Linters

- Rustfmt: <https://github.com/rust-lang/rustfmt#quick-start>
- Clippy: <https://github.com/rust-lang/rust-clippy#step-2-install-clippy>

## Security Audit

> ⚠️ _Make sure to follow the install instructions [here](#install-audit) on your first time_

It goes without saying, this is hugely important to use. Make sure to run this if there are any changes to the dependencies or `Cargo.lock` file.

❗ This is enforced by our Action jobs.

```sh
cargo audit
```

### Install Audit

- RustSec's Audit: <https://github.com/rustsec/rustsec/tree/main/cargo-audit#installation>

## End-to-End testing

It's strongly encouraged to run a full test to make sure interaction with external tools are not broken.
You can follow the [Local Development Setup](local_dev_setup.md) guide to perform this.

## How-to Update

We currently target the most current stable version of the rust toolchain. Periodically execute

```sh
rustup update
```
