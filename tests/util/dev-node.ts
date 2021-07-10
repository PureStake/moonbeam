import tcpPortUsed from "tcp-port-used";
import { spawn, ChildProcess } from "child_process";
import { BINARY_PATH, DISPLAY_LOG, MOONBEAM_LOG, SPAWNING_TIME, ANALYZER } from "./constants";
const debug = require("debug")("test:dev-node");

export async function findAvailablePorts() {
  const availablePorts = await Promise.all(
    [null, null, null].map(async (_, index) => {
      let selectedPort = 0;
      let port = 1024 + index * 20000 + (process.pid % 20000);
      let endingPort = 65535;
      while (!selectedPort && port < endingPort) {
        const inUse = await tcpPortUsed.check(port, "127.0.0.1");
        if (!inUse) {
          selectedPort = port;
        }
        port++;
      }
      if (!selectedPort) {
        throw new Error(`No available port`);
      }
      return selectedPort;
    })
  );

  return {
    p2pPort: availablePorts[0],
    rpcPort: availablePorts[1],
    wsPort: availablePorts[2],
  };
}

// Stores if the node has already started.
// It is used when a test file contains multiple describeDevMoonbeam. Those are
// executed within the same PID and so would generate a race condition if started
// at the same time.
let nodeStarted = false;

// This will start a moonbeam dev node, only 1 at a time (check every 100ms).
// This will prevent race condition on the findAvailablePorts which uses the PID of the process
export async function startMoonbeamDevNode(): Promise<{
  p2pPort: number;
  rpcPort: number;
  wsPort: number;
  runningNode: ChildProcess;
}> {
  while (nodeStarted) {
    // Wait 100ms to see if the node is free
    await new Promise((resolve) => {
      setTimeout(resolve, 100);
    });
  }
  nodeStarted = true;
  const { p2pPort, rpcPort, wsPort } = await findAvailablePorts();

  const moonbeamArgs = [
    `--execution=Native`, // Faster execution using native
    `--no-telemetry`,
    `--no-prometheus`,
    `--dev`,
    `--ethapi=txpool,debug,trace`,
    `--sealing=manual`,
    `-l${MOONBEAM_LOG}`,
    `--port=${p2pPort}`,
    `--rpc-port=${rpcPort}`,
    `--ws-port=${wsPort}`,
    `--tmp`,
  ];

  const [cmd, args] =
    ANALYZER == "valgrind"
      ? ["valgrind", ["--tool=callgrind", "-v", BINARY_PATH, ...moonbeamArgs]]
      : ANALYZER == "perf"
      ? ["perf", ["record", "-F", "997", "-g", BINARY_PATH, ...moonbeamArgs]]
      : [BINARY_PATH, moonbeamArgs];

  debug(`Starting dev node: --port=${p2pPort} --rpc-port=${rpcPort} --ws-port=${wsPort}`);

  const onProcessExit = function () {
    runningNode && runningNode.kill();
  };
  const onProcessInterrupt = function () {
    process.exit(2);
  };

  let runningNode: ChildProcess = null;
  process.once("exit", onProcessExit);
  process.once("SIGINT", onProcessInterrupt);
  runningNode = spawn(cmd, args);

  runningNode.once("exit", () => {
    process.removeListener("exit", onProcessExit);
    process.removeListener("SIGINT", onProcessInterrupt);
    nodeStarted = false;
    debug(`Exiting dev node: --port=${p2pPort} --rpc-port=${rpcPort} --ws-port=${wsPort}`);
  });

  runningNode.on("error", (err) => {
    if ((err as any).errno == "ENOENT") {
      console.error(
        `\x1b[31mMissing Moonbeam binary ` +
          `(${BINARY_PATH}).\nPlease compile the Moonbeam project\x1b[0m`
      );
    } else {
      console.error(err);
    }
    process.exit(1);
  });

  const binaryLogs = [];
  await new Promise<void>((resolve) => {
    const timer = setTimeout(() => {
      console.error(`\x1b[31m Failed to start Moonbeam Test Node.\x1b[0m`);
      console.error(`Command: ${cmd} ${args.join(" ")}`);
      console.error(`Logs:`);
      console.error(binaryLogs.map((chunk) => chunk.toString()).join("\n"));
      throw new Error("Failed to launch node");
    }, SPAWNING_TIME - 2000);

    const onData = async (chunk) => {
      if (DISPLAY_LOG) {
        console.log(chunk.toString());
      }
      binaryLogs.push(chunk);
      if (chunk.toString().match(/Development Service Ready/)) {
        clearTimeout(timer);
        if (!DISPLAY_LOG) {
          runningNode.stderr.off("data", onData);
          runningNode.stdout.off("data", onData);
        }
        resolve();
      }
    };
    runningNode.stderr.on("data", onData);
    runningNode.stdout.on("data", onData);
  });

  return { p2pPort, rpcPort, wsPort, runningNode };
}
