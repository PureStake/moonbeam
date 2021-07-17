# ![Moonbeam](media/moonbeam-cover.jpg)

## Contributions

Moonbeam primarily uses GitHub Pull Requests to coordinate code changes. If you wish to propose a
contribution, open a pull request and review the template, which explains how to document your
proposal.

You may also consider joining our [Discord server](https://discord.gg/PfpUATX) or
[Element room](https://app.element.io/#/room/#moonbeam:matrix.org) to discuss your changes.

### Generated Documentation

You can explore our [crate-level documentation](https://purestake.github.io/moonbeam).
This documentation is
automatically built and reflects the latest `master` commit.

### Code style

Moonbeam is following the
[Substrate code style](https://github.com/paritytech/substrate/blob/master/docs/STYLE_GUIDE.md).

In addition, we incorporate several tools to improve code quality. These are integrated into our CI
and are expected to pass before a PR is considered mergeable. They can also be run locally.

* [clippy](https://github.com/rust-lang/rust-clippy) - run with `cargo clippy --release --workspace`
* [rustfmt](https://github.com/rust-lang/rustfmt) - run with `cargo fmt -- --check`
* [editorconfig](https://editorconfig.org/) - integrate into your text editor / IDE
* [prettier](https://prettier.io/) - run with `npx prettier --check --ignore-path .gitignore '**/*.(yml|js|ts|json)'` (runs against `typescript` code)

### Directory Structure

The following is a list of directories of interest in development.

|Directory              |Purpose                                                                     |
| --------------------- | -------------------------------------------------------------------------- |
|client/                | Debug & Trace related code (rust)                                          |
|docker/                | Dockerfiles for running Moonbeam                                           |
|moonbeam-types-bundle/ | PolkadotJs types definitions for Moonbeam (typescript)                     |
|node/                  | Moonbeam's main node (rust)                                                |
|pallets/               | Moonmeam's Substrate runtime pallets (rust)                                |
|primitives/            | More Debug & Trace related code (rust)                                     |
|runtime/               | Moonbeam's runtime (on-chain) code (rust, compiled to WASM)                |
|scripts/               | Utilities for launching and interacting with a Moonbeam chain (typescript) |
|specs/                 | Spec files used to generate genesis for well-known Moonbeam networks       |
|tools/                 | Various tools generally related to development (typescript)                |

### PR labels conventions

Any PR must indicate whether the changes should be part of the runtime changelog or the binary changelog or neither.

If the changes are to be listed in the runtime changelog, associate the label `B7-runtimenoteworthy` with your PR.

If the changes should be listed in the binary changelog, associate the label `B5-clientnoteworthy` with your PR.

If the changes are not to be listed in any changelog, associate the label `B0-silent` with your PR.
