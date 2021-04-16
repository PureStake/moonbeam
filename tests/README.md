# Functional testing for Moonbeam

This folder contains a set of functional tests desgined for Moonbeam network.

It is written in typescript, using Mocha/Chai as Test framework.

## Test flow

Each group will start a dev service with the
[development spec](../node/src/chain_spec.rs) before executing the tests.

## Installation

```
npm install
```

## Run the tests

```
npm run test
```

and to print more information:

```
npm run test-with-logs
```

## Verbose mode

You can also add the node's logs to the output using the `MOONBEAM_LOG` env variable. Ex:

```
MOONBEAM_LOG="info,evm=trace,rpc=trace,ethereum=trace" npm run test
```

The Moonbeam node will be listening for RPC on port 19933, mostly to avoid
conflicts with already running Substrate nodes.
