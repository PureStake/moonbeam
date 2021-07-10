# Functional testing for Moonbeam

This folder contains a set of functional tests designed for Moonbeam network.

It is written in typescript, using Mocha/Chai as Test framework.

## Test flow

Each group will start a dev service with the
[development spec](../node/service/src/chain_spec) before executing the tests.

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

## Write Tests

### Add a new contract

- Add contract source code to `contracts/sources.ts`
- Run `npm run pre-build-contracts`=> This will generate the necessary abi and byte code
- Create your contract with
  `const { contract, rawTx } = await createContract(context.web3, "TestContract");`

## Verbose mode

You can also add the node's logs to the output using the `MOONBEAM_LOG` env variable. Ex:

```
MOONBEAM_LOG="warn,rpc=trace" npm run test
```

The test script will find available ports above 20000 in order to ensure that it doesn't conflict
with any other running services.

## Analyzer mode

The node supports being run with `perf` or with `valgrind`.  
However it requires to run only 1 test at a time

```
ANALYZER=valgrind npm run test -- --grep 'should allow optimized concurrent execution'
```

This will generate a file `callgrind.out.xxxxx`

```
ANALYZER=perf npm run test -- --grep 'should allow optimized concurrent execution'
```

This will generate a file `perf.data`

# Debugging a Moonbeam node

The repository contains a pre-configured debugger configuration for VSCode with the **CodeLLDB**
(`vadimcn.vscode-lldb`) extension.

Before debugging, you need to build the node with debug symbols with command
`RUSTFLAGS=-g cargo build --release` (available as a VSCode task). Then go in the **Debug** tab in
the left bar of VSCode and make sure **Launch Moonbeam Node (Linux)** is selected in the top
dropdown. **Build & Launch Moonbeam Node (Linux)** will trigger the build before launching the node.

To launch the debug session click on the green "play" arrow next to the dropdown. It will take some
time before the node starts, but the terminal containing the node output will appear when it is
really starting. The node is listening on ports 19931 (p2p), 19932 (rpc) and 19933 (ws).

You can explore the code and place a breakpoint on a line by left clicking on the left of the line
number. The execution will pause the next time this line is reached. The debug toolbar contains the
following buttons :

- Resume/Pause : Resume the execution if paused, pause the execution at the current location
  (pretty random) if running.
- Step over : Resume the execution until next line, or go one level up if the end of the current
  scope is reached.
- Step into : Resume the execution to go inside the immediatly next function call if any, otherwise
  step to next line.
- Step out : Resume the execution until the end of the scope is reached.
- Restart : Kill the program and start a new debuging session.
- Stop : Kill the program and end debugin session.

Breakpoints stay between debugging sessions. When multiple function calls are made on the same line,
multiple step into, step out, step into, ... can be requiered to go inside one of the chained
calls.

When paused, content of variables is showed in the debuging tab of VSCode. Some basic types are
displayed correctly (primitive types, Vec, Arc) but more complex types such as HashMap/BTreeMap
are not "smartly" displayed (content of the struct is shown by mapping is hidden in the complexity
of the implementation).

## Running Typescript tests with a debug node

By setting the environement variable `DEBUG_MODE=true`, the Typescript tests will not spawn its
own node and instead will connect to an external node running on ports 19931/19932/19933, which
are the ports used by the debug node.

A VSCode test allow to quickly run the `test-single` test in debug mode. To run another test,
change the command in the `package.json`. Note that you should restart the node after running
one test file.
