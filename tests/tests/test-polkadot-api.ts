import { expect } from "chai";
import { Keyring } from "@polkadot/keyring";
import { step } from "mocha-steps";

import { createAndFinalizeBlock, describeWithMoonbeam } from "./util";
import { AnyTuple, IEvent } from "@polkadot/types/types";
import { GENESIS_ACCOUNT, GENESIS_ACCOUNT_PRIVATE_KEY } from "./constants";

describeWithMoonbeam("Moonbeam Polkadot API", `simple-specs.json`, (context) => {
  step("api can retrieve last header", async function () {
    const lastHeader = await context.polkadotApi.rpc.chain.getHeader();
    expect(Number(lastHeader.number) >= 0).to.be.true;
  });

  step("api can retrieve last block", async function () {
    const signedBlock = await context.polkadotApi.rpc.chain.getBlock();
    expect(signedBlock.block.header.number.toNumber() >= 0).to.be.true;
  });

  const TEST_ACCOUNT_2 = "0x1111111111111111111111111111111111111112";

  step("transfer from polkadotjs should appear in ethereum", async function () {
    this.timeout(30000);

    const keyring = new Keyring({ type: "ethereum" });
    const testAccount = await keyring.addFromUri(GENESIS_ACCOUNT_PRIVATE_KEY, null, "ethereum");
    try {
      let hash = await context.polkadotApi.tx.balances
        .transfer(TEST_ACCOUNT_2, 123)
        .signAndSend(testAccount);
    } catch (e) {
      expect(false, "error during polkadot api transfer" + e);
    }
    // TODO: do some testing with the hash
    await createAndFinalizeBlock(context.polkadotApi);
    let res = await context.polkadotApi.query.balances.account(TEST_ACCOUNT_2);
    console.log("res", res);
    expect(res).to.eq("123");
    expect(await context.web3.eth.getBalance(TEST_ACCOUNT_2)).to.equal("123");
  });

  step("read extrinsic information", async function () {
    const signedBlock = await context.polkadotApi.rpc.chain.getBlock();
    expect(signedBlock.block.header.number.toNumber() >= 0).to.be.true;

    // Expecting 4 extrinsics so far:
    // timestamp, author, the parachain validation data and the balances transfer.
    expect(signedBlock.block.extrinsics).to.be.of.length(4);

    signedBlock.block.extrinsics.forEach((ex, index) => {
      const {
        isSigned,
        method: { args, method, section },
      } = ex;
      const message = `${section}.${method}(${args.map((a) => a.toString()).join(", ")})`;
      switch (index) {
        case 0:
          expect(message.substring(0, 13)).to.eq(`timestamp.set`);
          break;
        case 1:
          expect(message.substring(0, 33)).to.eq(`parachainSystem.setValidationData`);
          break;
        case 2:
          expect(message).to.eq(
            `authorInherent.setAuthor(0x6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b)`
          );
          break;
        case 3:
          expect(ex.signer.toString().toLocaleLowerCase()).to.eq(GENESIS_ACCOUNT);
          expect(message).to.eq(
            `balances.transfer(0x1111111111111111111111111111111111111112, 123)`
          );
          break;
        default:
          throw new Error(`Unexpected extrinsic: ${message}`);
      }
    });
  });

  step("read extrinsic events", async function () {
    const signedBlock = await context.polkadotApi.rpc.chain.getBlock();
    const allRecords = await context.polkadotApi.query.system.events.at(
      signedBlock.block.header.hash
    );

    // map between the extrinsics and events
    signedBlock.block.extrinsics.forEach(({ method: { method, section } }, index) => {
      // filter the specific events based on the phase and then the
      // index of our extrinsic in the block
      const events: IEvent<AnyTuple>[] = allRecords
        .filter(({ phase }) => phase.isApplyExtrinsic && phase.asApplyExtrinsic.eq(index))
        .map(({ event }) => event);

      switch (index) {
        // First 3 events:
        // timestamp.set:: system.ExtrinsicSuccess
        // parachainUpgrade.setValidationData:: system.ExtrinsicSuccess
        // authorInherent.setAuthor:: system.ExtrinsicSuccess
        case 0:
        case 1:
        case 2:
          expect(
            events.length === 1 && context.polkadotApi.events.system.ExtrinsicSuccess.is(events[0])
          ).to.be.true;
          break;
        // Fourth event: balances.transfer:: system.NewAccount, balances.Endowed, balances.Transfer,
        // system.ExtrinsicSuccess
        case 3:
          expect(events.length === 4);
          expect(context.polkadotApi.events.system.NewAccount.is(events[0])).to.be.true;
          expect(context.polkadotApi.events.balances.Endowed.is(events[1])).to.be.true;
          expect(context.polkadotApi.events.balances.Transfer.is(events[2])).to.be.true;
          expect(context.polkadotApi.events.system.ExtrinsicSuccess.is(events[3])).to.be.true;
          break;
        default:
          throw new Error(`Unexpected extrinsic`);
      }
    });
  });
});
