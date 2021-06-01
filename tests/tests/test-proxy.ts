import { expect } from "chai";
import { customWeb3Request } from "../util/providers";
import { describeDevMoonbeam } from "../util/setup-dev-tests";
import Keyring from "@polkadot/keyring";
import {
  ALITH_PRIVATE_KEY,
  BALTATHAR_PRIVATE_KEY,
  CHARLETH_PRIVATE_KEY,
  CHARLETH_ADDRESS,
} from "../util/constants";

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

// In these tests Alith will allow Baltathar to perform calls on her behalf.
// Charleth is used as a target account when making transfers.

describeDevMoonbeam("Pallet proxy", (context) => {
  let alith;
  let baltathar;
  let charleth;
  before("Setup: prepare keyring", async function () {
    const keyring = new Keyring({ type: "ethereum" });
    alith = await keyring.addFromUri(ALITH_PRIVATE_KEY, null, "ethereum");
    baltathar = keyring.addFromUri(BALTATHAR_PRIVATE_KEY, null, "ethereum");
    charleth = keyring.addFromUri(CHARLETH_PRIVATE_KEY, null, "ethereum");
  });

  it("shouldn't accept unknown proxy", async function () {
    const balance_before = await await context.web3.eth.getBalance(CHARLETH_ADDRESS);

    // Proxy call
    const unsub = await context.polkadotApi.tx.proxy
      .proxy(alith.address, null, context.polkadotApi.tx.balances.transfer(charleth.address, 100))
      .signAndSend(baltathar, ({ events = [], status }) => {
        if (status.isInBlock) {
          // Check proxy call failed.
          expect(events[3].event.method).to.be.eq("ExtrinsicFailed");
          unsub();
        }
      });

    await context.createBlock();
    await delay(500);

    const balance_after = await await context.web3.eth.getBalance(CHARLETH_ADDRESS);

    // Check target balance didn't change.
    expect(balance_after).to.be.eq(balance_before);
  });

  it("should accept known proxy", async function () {
    const balance_before = await await context.web3.eth.getBalance(CHARLETH_ADDRESS);

    // Allow proxy
    await context.polkadotApi.tx.proxy.addProxy(baltathar.address, "Any", 0).signAndSend(alith);

    // Proxy call
    const unsub = await context.polkadotApi.tx.proxy
      .proxy(alith.address, null, context.polkadotApi.tx.balances.transfer(charleth.address, 100))
      .signAndSend(baltathar, ({ events = [], status }) => {
        if (status.isInBlock) {
          // Check proxy call succeeded.
          expect(events[3].event.method).to.be.eq("ExtrinsicSuccess");
          unsub();
        }
      });

    await context.createBlock();
    await delay(500);

    const balance_after = await await context.web3.eth.getBalance(CHARLETH_ADDRESS);

    // Check target balance changed with correct amount.
    expect(BigInt(balance_after)).to.be.eq(BigInt(balance_before) + 100n);
  });

  it("should remove proxy", async function () {
    const balance_before = await context.web3.eth.getBalance(CHARLETH_ADDRESS);

    // Remove proxy
    await context.polkadotApi.tx.proxy.removeProxy(baltathar.address, "Any", 0).signAndSend(alith);

    // Proxy call
    const unsub = await context.polkadotApi.tx.proxy
      .proxy(alith.address, null, context.polkadotApi.tx.balances.transfer(charleth.address, 100))
      .signAndSend(baltathar, ({ events = [], status }) => {
        if (status.isInBlock) {
          // Check proxy call failed.
          expect(events[1].event.method).to.be.eq("ExtrinsicFailed");
          unsub();
        }
      });

    await context.createBlock();
    await delay(500);

    const balance_after = await context.web3.eth.getBalance(CHARLETH_ADDRESS);

    // Check target balance didn't change.
    expect(balance_after).to.be.eq(balance_before);
  });

  it("should refuse direct call to known delayed proxy", async function () {
    const balance_before = await context.web3.eth.getBalance(CHARLETH_ADDRESS);

    // Allow proxy (with delay)
    await context.polkadotApi.tx.proxy.addProxy(baltathar.address, "Any", 2).signAndSend(alith);

    // Proxy call
    const unsub = await context.polkadotApi.tx.proxy
      .proxy(alith.address, null, context.polkadotApi.tx.balances.transfer(charleth.address, 100))
      .signAndSend(baltathar, ({ events = [], status }) => {
        if (status.isInBlock) {
          // Check proxy call failed.
          expect(events[1].event.method).to.be.eq("ExtrinsicFailed");
          unsub();
        }
      });

    await context.createBlock();
    await delay(500);

    const balance_after = await context.web3.eth.getBalance(CHARLETH_ADDRESS);

    // Check target balance didn't change.
    expect(balance_after).to.be.eq(balance_before);
  });

  let transfer_tx;
  it("should be able to announce a call", async function () {
    transfer_tx = await context.polkadotApi.tx.balances.transfer(charleth.address, 100);

    // Proxy announcement
    const unsub = await context.polkadotApi.tx.proxy
      .announce(alith.address, transfer_tx.hash)
      .signAndSend(baltathar, ({ events = [], status }) => {
        if (status.isInBlock) {
          // Check proxy call succeeded.
          expect(events[1].event.method).to.be.eq("Announced");
          expect(events[3].event.method).to.be.eq("ExtrinsicSuccess");
          unsub();
        }
      });

    await context.createBlock();
    await delay(500);
  });

  it("should refuse early announced call", async function () {
    const balance_before = await context.web3.eth.getBalance(CHARLETH_ADDRESS);

    // Proxy call
    const unsub = await context.polkadotApi.tx.proxy
      .proxyAnnounced(alith.address, baltathar.address, null, transfer_tx)
      .signAndSend(baltathar, ({ events = [], status }) => {
        if (status.isInBlock) {
          events.forEach((value) => console.log(value.event.method));
          // Check proxy call failed.
          expect(events[1].event.method).to.be.eq("ExtrinsicFailed");
          unsub();
        }
      });

    await context.createBlock();
    await delay(500);

    const balance_after = await context.web3.eth.getBalance(CHARLETH_ADDRESS);

    // Check target balance didn't change.
    expect(balance_after).to.be.eq(balance_before);
  });

  it("should accept on-time announced call", async function () {
    const balance_before = await context.web3.eth.getBalance(CHARLETH_ADDRESS);

    await context.createBlock();

    // Proxy call
    const unsub = await context.polkadotApi.tx.proxy
      .proxyAnnounced(alith.address, baltathar.address, null, transfer_tx)
      .signAndSend(baltathar, ({ events = [], status }) => {
        if (status.isInBlock) {
          events.forEach((value) => console.log(value.event.method));
          // Check proxy call succeeded.
          expect(events[1].event.method).to.be.eq("ExtrinsicFailed"); // should be ExtrinsicSucceed

          unsub();
        }
      });

    await context.createBlock();
    await delay(500);

    const balance_after = await context.web3.eth.getBalance(CHARLETH_ADDRESS);

    // Check target balance didn't change.
    expect(balance_after).to.be.eq(balance_before);
  });
});
