/**
 *  Script to launch 2 relay and 2 parachain nodes.
 *  It contains pre-registered versions to allow easy run using Docker.
 *
 *  ports can be given using --port-prefix xx (default 34) using the following rule:
 *  - relay 1 - p2p (p2p: XX000, rpcPort: XX001, wsPort: XX002)
 *  - relay 2 - p2p (p2p: XX010, rpcPort: XX011, wsPort: XX012)
 *  - para 1 - p2p (p2p: XX100, rpcPort: XX101, wsPort: XX102)
 *  - para 2 - p2p (p2p: XX110, rpcPort: XX111, wsPort: XX112)
 *
 */

import yargs from "yargs";
import * as fs from "fs";
import * as path from "path";
import * as child_process from "child_process";
import { killAll, run } from "polkadot-launch";

// Description of the network to launch
type NetworkConfig = {
  // From which docker to take the binary
  docker?: string;
  // To use instead of docker to run local binary
  binary?: string;
  // What chain to run
  chain: string;
};

// Description of the parachain network
type ParachainConfig = NetworkConfig & {
  // Which relay (name) config to use
  relay: string;
};

const parachains: { [name: string]: ParachainConfig } = {
  "moonriver-genesis": {
    relay: "kusama-9040",
    chain: "moonriver-local",
    docker: "purestake/moonbeam:moonriver-genesis",
  },
  "moonriver-genesis-fast": {
    relay: "rococo-9004",
    chain: "moonriver-local",
    docker: "purestake/moonbeam:sha-153c4c4a",
  },
  "moonbase-0.8.2": {
    relay: "rococo-9004",
    chain: "moonbase-local",
    docker: "purestake/moonbeam:v0.8.2",
  },
  "moonbase-0.8.1": {
    relay: "rococo-9004",
    chain: "moonbase-local",
    docker: "purestake/moonbeam:v0.8.1",
  },
  "moonbase-0.8.0": {
    relay: "rococo-9001",
    chain: "moonbase-local",
    docker: "purestake/moonbeam:v0.8.0",
  },
  "moonbase-0.9.2": {
    relay: "rococo-9004",
    chain: "moonbase-local",
    docker: "purestake/moonbeam:v0.9.2",
  },
  "moonbase-0.9.4": {
    relay: "rococo-9004",
    chain: "moonbase-local",
    docker: "purestake/moonbeam:v0.9.4",
  },
  local: {
    relay: "rococo-9004",
    chain: "moonbase-local",
    binary: "../target/release/moonbeam",
  },
};
const parachainNames = Object.keys(parachains);

const relays: { [name: string]: NetworkConfig } = {
  "kusama-9030": {
    docker: "purestake/moonbase-relay-testnet:sha-aa386760",
    chain: "kusama-local",
  },
  "kusama-9040": {
    docker: "purestake/moonbase-relay-testnet:sha-2f28561a",
    chain: "kusama-local",
  },
  "kusama-9030-fast": {
    docker: "purestake/moonbase-relay-testnet:sha-832cc0af",
    chain: "kusama-local",
  },
  "kusama-9040-fast": {
    docker: "purestake/moonbase-relay-testnet:sha-2239072e",
    chain: "kusama-local",
  },
  "rococo-9001": {
    docker: "purestake/moonbase-relay-testnet:sha-86a45114",
    chain: "rococo-local",
  },
  "rococo-9003": {
    docker: "purestake/moonbase-relay-testnet:sha-aa386760",
    chain: "rococo-local",
  },
  "rococo-9004": {
    docker: "purestake/moonbase-relay-testnet:sha-2f28561a",
    chain: "rococo-local",
  },
  "westend-9030": {
    docker: "purestake/moonbase-relay-testnet:sha-aa386760",
    chain: "westend-local",
  },
  "westend-9040": {
    docker: "purestake/moonbase-relay-testnet:sha-2f28561a",
    chain: "westend-local",
  },
  local: {
    binary: "../../polkadot/target/release/polkadot",
    chain: "rococo-local",
  },
};
const relayNames = Object.keys(relays);

function start() {
  const argv = yargs(process.argv.slice(2))
    .usage("Usage: npm run launch [args]")
    .version("1.0.0")
    .options({
      parachain: {
        type: "string",
        choices: Object.keys(parachains),
        default: "local",
        describe: "which parachain configuration to run",
      },
      "parachain-chain": {
        type: "string",
        describe: "overrides parachain chain/runtime",
      },
      "parachain-id": { type: "number", default: 1000, describe: "overrides parachain-id" },
      relay: {
        type: "string",
        choices: Object.keys(relays),
        describe: "overrides relay configuration",
      },
      "relay-chain": {
        type: "string",
        choices: [
          "rococo",
          "westend",
          "kusama",
          "polkadot",
          "rococo-local",
          "westend-local",
          "kusama-local",
          "polkadot-local",
        ],
        describe: "overrides relay chain/runtime",
      },
      "port-prefix": {
        type: "number",
        default: 34,
        check: (port) => port >= 0 && port <= 64,
        describe: "provides port prefix for nodes",
      },
    })
    .help().argv;

  const portPrefix = argv["port-prefix"] || 34;
  const startingPort = portPrefix * 1000;
  const parachainName = argv.parachain.toString();
  const parachain = parachains[parachainName];
  const parachainChain = argv["parachain-chain"] || parachain.chain;

  const relayName = argv.relay || parachain.relay;

  if (!relayName || !relayNames.includes(relayName)) {
    console.error(`Invalid relay name: ${relayName}`);
    console.error(`Expected one of: ${relayNames.join(", ")}`);
    return;
  }

  const relay = relays[relayName];
  const relayChain = argv["relay-chain"] || relay.chain;

  console.log(
    `🚀 Relay:     ${relayName.padEnd(20)} - ${relay.docker || relay.binary} (${relayChain})`
  );

  let parachainBinary;
  if (parachain.binary) {
    parachainBinary = parachain.binary;
    const parachainPath = path.join(__dirname, parachain.binary);
    if (!fs.existsSync(parachainPath)) {
      console.log(`     Missing ${parachainPath}`);
      return;
    }
  } else {
    if (process.platform != "linux") {
      console.log(
        `docker binaries are only supported on linux. Use "local" config for compiled binaries`
      );
      return;
    }
    parachainBinary = `build/${parachainName}/moonbeam`;
    const parachainPath = path.join(__dirname, `build/${parachainName}/moonbeam`);
    if (!fs.existsSync(parachainPath)) {
      console.log(`     Missing ${parachainBinary} locally, downloading it...`);
      child_process.execSync(`mkdir -p ${path.dirname(parachainPath)} && \
          docker create --name moonbeam-tmp ${parachain.docker} && \
          docker cp moonbeam-tmp:/moonbeam/moonbeam ${parachainPath} && \
          docker rm moonbeam-tmp`);
      console.log(`     ${parachainBinary} downloaded !`);
    }
  }
  console.log(
    `🚀 Parachain: ${parachainName.padEnd(20)} - ${
      parachain.docker || parachain.binary
    } (${parachainChain})`
  );

  let relayBinary;
  if (relay.binary) {
    relayBinary = relay.binary;
    const relayPath = path.join(__dirname, relay.binary);
    if (!fs.existsSync(relayPath)) {
      console.log(`     Missing ${relayPath}`);
      return;
    }
  } else {
    if (process.platform != "linux") {
      console.log(
        `docker binaries are only supported on linux. Use "local" config for compiled binaries`
      );
      return;
    }
    relayBinary = `build/${relayName}/polkadot`;
    const relayPath = path.join(__dirname, `build/${relayName}/polkadot`);
    if (!fs.existsSync(relayPath)) {
      console.log(`     Missing ${relayBinary} locally, downloading it...`);
      child_process.execSync(`mkdir -p ${path.dirname(relayPath)} && \
          docker create --name polkadot-tmp ${relay.docker} && \
          docker cp polkadot-tmp:/usr/local/bin/polkadot ${relayPath} && \
          docker rm polkadot-tmp`);
      console.log(`     ${relayBinary} downloaded !`);
    }
  }
  console.log("");

  let launchConfig = launchTemplate;
  launchConfig.relaychain.bin = relayBinary;
  launchConfig.relaychain.chain = relayChain;
  launchConfig.parachains[0].bin = parachainBinary;
  launchConfig.parachains[0].chain = parachainChain;

  launchConfig.parachains[0].id = argv["parachain-id"] || 1000;

  launchConfig.relaychain.nodes[0].port = startingPort;
  launchConfig.relaychain.nodes[0].rpcPort = startingPort + 1;
  launchConfig.relaychain.nodes[0].wsPort = startingPort + 2;

  launchConfig.relaychain.nodes[1].port = startingPort + 10;
  launchConfig.relaychain.nodes[1].rpcPort = startingPort + 11;
  launchConfig.relaychain.nodes[1].wsPort = startingPort + 12;

  launchConfig.parachains[0].nodes[0].port = startingPort + 100;
  launchConfig.parachains[0].nodes[0].rpcPort = startingPort + 101;
  launchConfig.parachains[0].nodes[0].wsPort = startingPort + 102;

  launchConfig.parachains[0].nodes[1].port = startingPort + 110;
  launchConfig.parachains[0].nodes[1].rpcPort = startingPort + 111;
  launchConfig.parachains[0].nodes[1].wsPort = startingPort + 112;

  const knownRelayChains = ["kusama", "westend", "rococo", "polkadot"]
    .map((network) => [`${network}`, `${network}-local`, `${network}-dev`])
    .flat();

  // In case the chain is a spec file
  if (!knownRelayChains.includes(launchConfig.relaychain.chain)) {
    delete launchConfig.relaychain.genesis;
  } else if (launchConfig.relaychain.chain.startsWith("rococo")) {
    // To support compatibility with rococo
    (launchConfig.relaychain.genesis.runtime as any).runtime_genesis_config = {
      ...launchConfig.relaychain.genesis.runtime,
    };
    for (let key of Object.keys(launchConfig.relaychain.genesis.runtime)) {
      if (key != "runtime_genesis_config") {
        delete launchConfig.relaychain.genesis.runtime[key];
      }
    }
  }

  // Kill all processes when exiting.
  process.on("exit", function () {
    killAll();
  });

  // Handle ctrl+c to trigger `exit`.
  process.on("SIGINT", function () {
    process.exit(2);
  });

  run(__dirname, launchConfig);
}

const launchTemplate = {
  relaychain: {
    bin: "...",
    chain: "...",
    nodes: [
      {
        name: "alice",
        port: 0,
        rpcPort: 1,
        wsPort: 2,
      },
      {
        name: "bob",
        port: 10,
        rpcPort: 11,
        wsPort: 12,
      },
    ],
    genesis: {
      runtime: {
        parachainsConfiguration: {
          config: {
            validation_upgrade_frequency: 1,
            validation_upgrade_delay: 1,
          },
        },
      },
    },
  },
  parachains: [
    {
      bin: "...",
      id: 1000,
      balance: "1000000000000000000000",
      chain: "...",
      nodes: [
        {
          port: 100,
          rpcPort: 101,
          wsPort: 102,
          name: "alice",
          flags: [
            "--log=info,rpc=trace,evm=trace,ethereum=trace",
            "--unsafe-rpc-external",
            "--rpc-cors=all",
            "--",
            "--execution=wasm",
          ],
        },
        {
          port: 110,
          rpcPort: 111,
          wsPort: 112,
          name: "bob",
          flags: [
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
  types: {
    Address: "MultiAddress",
    LookupSource: "MultiAddress",
    RoundIndex: "u32",
  },
  finalization: true,
};

start();
