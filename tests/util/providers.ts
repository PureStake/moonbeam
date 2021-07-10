import Web3 from "web3";
import { ApiPromise, WsProvider } from "@polkadot/api";
import { typesBundle } from "../../moonbeam-types-bundle";
import { JsonRpcResponse } from "web3-core-helpers";
import { ethers } from "ethers";
import { GENESIS_ACCOUNT_PRIVATE_KEY } from "./constants";
import { Subscription as Web3Subscription } from "web3-core-subscriptions";
import { BlockHeader } from "web3-eth";
import { Log } from "web3-core";
import * as http from "http";
const debug = require("debug")("test:providers");

export async function customWeb3Request(web3: Web3, method: string, params: any[]) {
  return new Promise<JsonRpcResponse>((resolve, reject) => {
    (web3.currentProvider as any).send(
      {
        jsonrpc: "2.0",
        id: 1,
        method,
        params,
      },
      (error: Error | null, result?: JsonRpcResponse) => {
        if (error) {
          reject(
            `Failed to send custom request (${method} (${params.join(",")})): ${
              error.message || error.toString()
            }`
          );
        }
        resolve(result);
      }
    );
  });
}

const agent = new http.Agent({
  keepAlive: true,
  maxSockets: 10,
  maxFreeSockets: 10,
  maxTotalSockets: 10,
});

const getDuration = (start: bigint, end: bigint) => {
  return Number((end - start) / 1000000n);
};

export async function customRawRequest(host: string, port: number, method: string, params: any[]) {
  return new Promise<string>((resolve, reject) => {
    const timings = {
      startAt: process.hrtime.bigint(),
      dnsLookupAt: undefined,
      tcpConnectionAt: undefined,
      tlsHandshakeAt: undefined,
      firstByteAt: undefined,
      endAt: undefined,
    };
    const responseBody = [];
    const req = http.request(
      {
        headers: {
          "Content-Type": "application/json",
        },
        method: "POST",
        agent,
        host,
        port,
      },
      function (res) {
        res.once("readable", () => {
          timings.firstByteAt = process.hrtime.bigint();
        });
        res.on("data", (chunk) => {
          responseBody.push(chunk);
        });
        res.on("end", () => {
          timings.endAt = process.hrtime.bigint();
          debug(`Request done`, {
            tcpConn: getDuration(
              timings.dnsLookupAt || timings.startAt,
              timings.tcpConnectionAt || timings.startAt
            ),
            firstByte: getDuration(
              timings.tlsHandshakeAt || timings.tcpConnectionAt || timings.startAt,
              timings.firstByteAt
            ),
            transfer: getDuration(timings.firstByteAt, timings.endAt),
            total: getDuration(timings.startAt, timings.endAt),
          });
          resolve(responseBody.join(""));
        });
      }
    );

    req.on("socket", (socket) => {
      socket.on("lookup", () => {
        timings.dnsLookupAt = process.hrtime.bigint();
      });
      socket.on("connect", () => {
        timings.tcpConnectionAt = process.hrtime.bigint();
      });
      socket.on("secureConnect", () => {
        timings.tlsHandshakeAt = process.hrtime.bigint();
      });
    });

    req.on("error", function (e) {
      reject("problem with request: " + e.message);
    });
    // write data to request body
    req.write(
      JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: method,
        params: params || [],
      })
    );
    req.end();
  });
}

// Extra type because web3 is not well typed
export interface Subscription<T> extends Web3Subscription<T> {
  once: (type: "data" | "connected", handler: (data: T) => void) => Subscription<T>;
}

// Little helper to hack web3 that are not complete.
export function web3Subscribe(web3: Web3, type: "newBlockHeaders"): Subscription<BlockHeader>;
export function web3Subscribe(web3: Web3, type: "pendingTransactions"): Subscription<string>;
export function web3Subscribe(web3: Web3, type: "logs", params: {}): Subscription<Log>;
export function web3Subscribe(
  web3: Web3,
  type: "newBlockHeaders" | "pendingTransactions" | "logs",
  params?: any
) {
  return (web3.eth as any).subscribe(...[].slice.call(arguments, 1));
}

export type EnhancedWeb3 = Web3 & {
  customRequest: (method: string, params: any[]) => Promise<JsonRpcResponse>;
};

export const provideWeb3Api = async (port: number, protocol: "ws" | "http" = "http") => {
  const web3 =
    protocol == "ws"
      ? new Web3(`ws://localhost:${port}`) // TODO: restore support for
      : new Web3(`http://localhost:${port}`);

  // Adding genesis account for convenience
  web3.eth.accounts.wallet.add(GENESIS_ACCOUNT_PRIVATE_KEY);

  // Hack to add customRequest method.
  (web3 as any).customRequest = (method: string, params: any[]) =>
    customWeb3Request(web3, method, params);

  return web3 as EnhancedWeb3;
};

export const providePolkadotApi = async (port: number) => {
  return await ApiPromise.create({
    initWasm: false,
    provider: new WsProvider(`ws://localhost:${port}`),
    typesBundle: typesBundle as any,
  });
};

export const provideEthersApi = async (port: number) => {
  return new ethers.providers.JsonRpcProvider(`http://localhost:${port}`);
};
