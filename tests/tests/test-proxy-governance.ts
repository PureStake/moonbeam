import { expect } from "chai";
import Keyring from "@polkadot/keyring";
import {
  ALITH_PRIV_KEY,
  DOROTHY,
  DOROTHY_PRIV_KEY,
  ETHAN,
  ETHAN_PRIVKEY,
  GENESIS_ACCOUNT_BALANCE,
  PROPOSAL_AMOUNT,
  VOTE_AMOUNT,
} from "../util/constants";
import { describeDevMoonbeam } from "../util/setup-dev-tests";

describeDevMoonbeam("Proxing governance", (context) => {
  it("should be able to vote on behalf of the delegate account", async function () {
    this.timeout(40_000);

    const keyring = new Keyring({ type: "ethereum" });

    const alith = await keyring.addFromUri(ALITH_PRIV_KEY, null, "ethereum");
    const dorothy = await keyring.addFromUri(DOROTHY_PRIV_KEY, null, "ethereum");
    const ethan = await keyring.addFromUri(ETHAN_PRIVKEY, null, "ethereum");

    // Dorothy add proxy rigth to ethan for governance only
    await context.polkadotApi.tx.proxy.addProxy(ETHAN, "Governance", 0).signAndSend(dorothy);

    // Alith submit a proposal
    let proposalHash = "0xf3d039875302d49d52fb1af6877a2c46bc55b004afb8130f94dd9d0489ca3185";
    await context.polkadotApi.tx.democracy
      .propose(proposalHash, PROPOSAL_AMOUNT)
      .signAndSend(alith);
    await context.createBlock();

    // Verify that Alith proposal is registered
    const publicPropCount = await context.polkadotApi.query.democracy.publicPropCount();
    expect(publicPropCount.toHuman()).to.equal("1");

    /// Wait launchPeriod elapsed
    let launchPeriod = await context.polkadotApi.consts.democracy.launchPeriod;
    for (let i = 0; i < Number(launchPeriod); i++) {
      await context.createBlock();
    }

    // Verify that one referundum is triggered
    let referendumCount = await context.polkadotApi.query.democracy.referendumCount();
    expect(referendumCount.toHuman()).to.equal("1");

    // Ethan vote as Dorothy
    let voteCall = context.polkadotApi.tx.democracy.vote(0, {
      Standard: { balance: VOTE_AMOUNT, vote: { aye: true, conviction: 1 } },
    });
    await context.polkadotApi.tx.proxy.proxy(DOROTHY, "Governance", voteCall).signAndSend(ethan);
    await context.createBlock();

    // Verify that dorothy tokens are used
    let dorothyAccountData = await context.polkadotApi.query.system.account(DOROTHY);
    expect((dorothyAccountData.toHuman() as any).data.free).to.equal("1.2089 MUNIT");

    // Verify that vote is registered
    const referendumInfoOf = await context.polkadotApi.query.democracy.referendumInfoOf(0);
    expect((referendumInfoOf.toHuman() as any).Ongoing.proposalHash).to.equal(proposalHash);
    expect((referendumInfoOf.toHuman() as any).Ongoing.tally.ayes).to.equal("10.0000 UNIT");
    expect((referendumInfoOf.toHuman() as any).Ongoing.tally.turnout).to.equal("10.0000 UNIT");
  });
});
