import tcpPortUsed from "tcp-port-used";
import path from "path";
import { killAll, run } from "polkadot-launch";
import { BINARY_PATH, RELAY_BINARY_PATH, DISPLAY_LOG, SPAWNING_TIME } from "./constants";
const debug = require("debug")("test:para-node");

export async function findAvailablePorts() {
  const availablePorts = await Promise.all(
    new Array(3 * 4).fill(0).map(async (_, index) => {
      let selectedPort = 0;
      let port = 1024 + index * 2000 + (process.pid % 2000);
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

  return new Array(4).fill(0).map((_, index) => ({
    p2pPort: availablePorts[index * 3 + 0],
    rpcPort: availablePorts[index * 3 + 1],
    wsPort: availablePorts[index * 3 + 2],
  }));
}

// Stores if the node has already started.
// It is used when a test file contains multiple describeDevMoonbeam. Those are
// executed within the same PID and so would generate a race condition if started
// at the same time.
let nodeStarted = false;

export type ParachainOptions = {
  chain:
    | "moonbase-local"
    | "moonshadow-local"
    | "moonriver-local"
    | "moonbeam-local"
    | "moonbase"
    | "moonshadow"
    | "moonriver"
    | "moonbeam";
  relaychain?: "rococo-local" | "westend-local" | "kusama-local" | "polkadot-local";
};

// This will start a parachain node, only 1 at a time (check every 100ms).
// This will prevent race condition on the findAvailablePorts which uses the PID of the process
export async function startParachainNodes(options: ParachainOptions): Promise<{
  p2pPort: number;
  rpcPort: number;
  wsPort: number;
}> {
  while (nodeStarted) {
    // Wait 100ms to see if the node is free
    await new Promise((resolve) => {
      setTimeout(resolve, 100);
    });
  }
  const relaychain = options.relaychain || "rococo-local";
  nodeStarted = true;
  // Each node will have 3 ports. There are 4 nodes total (so 12 ports)
  const ports = await findAvailablePorts();

  const launchConfig = {
    relaychain: {
      bin: RELAY_BINARY_PATH,
      chain: relaychain,
      nodes: [
        {
          name: "alice",
          port: ports[0].p2pPort,
          rpcPort: ports[0].rpcPort,
          wsPort: ports[0].wsPort,
        },
        {
          name: "bob",
          port: ports[1].p2pPort,
          rpcPort: ports[1].rpcPort,
          wsPort: ports[1].wsPort,
        },
      ],
      genesis: {
        runtime: {
          runtime_genesis_config: {
            parachainsConfiguration: {
              config: {
                validation_upgrade_frequency: 1,
                validation_upgrade_delay: 1,
              },
            },
          },
        },
      },
    },
    parachains: [
      {
        bin: BINARY_PATH,
        id: 1000,
        chain: options.chain,
        nodes: [
          {
            port: ports[2].p2pPort,
            rpcPort: ports[2].rpcPort,
            wsPort: ports[2].wsPort,
            name: "alice",
            flags: [
              `--no-telemetry`,
              `--no-prometheus`,
              "--log=info,rpc=trace,evm=trace,ethereum=trace",
              "--unsafe-rpc-external",
              "--rpc-cors=all",
              "--",
              "--execution=wasm",
            ],
          },
          {
            port: ports[3].p2pPort,
            rpcPort: ports[3].rpcPort,
            wsPort: ports[3].wsPort,
            name: "bob",
            flags: [
              `--no-telemetry`,
              `--no-prometheus`,
              "--log=info,rpc=trace,evm=trace,ethereum=trace",
              "--unsafe-rpc-external",
              "--rpc-cors=all",
              "--",
              "--execution=wasm",
            ],
          },
        ],
      },
    ],
    simpleParachains: [],
    hrmpChannels: [],
    finalization: true,
  };

  const onProcessExit = function () {
    killAll;
  };
  const onProcessInterrupt = function () {
    process.exit(2);
  };

  process.once("exit", onProcessExit);
  process.once("SIGINT", onProcessInterrupt);

  await run(path.join(__dirname, "../"), launchConfig);

  return {
    p2pPort: ports[2].p2pPort,
    rpcPort: ports[2].rpcPort,
    wsPort: ports[2].wsPort,
  };
}

export async function stopParachainNodes() {
  killAll();
  await new Promise((resolve) => {
    // TODO: improve
    setTimeout(resolve, 2000);
  });
}
