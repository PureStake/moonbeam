import { Keyring } from "@polkadot/api";
import { KeyringPair } from "@polkadot/keyring/types";
import { expect } from "chai";
import Web3 from "web3";
import { Account } from "web3-core";
import { formatBalance } from "@polkadot/util";
import type { SubmittableExtrinsic } from "@polkadot/api/promise/types";
import { blake2AsHex } from "@polkadot/util-crypto";
import {
  GENESIS_ACCOUNT,
  GENESIS_ACCOUNT_BALANCE,
  GENESIS_ACCOUNT_PRIVATE_KEY,
  GLMR,
  ALITH_PRIV_KEY,
  ALITH,
  PROPOSAL_AMOUNT,
  VOTE_AMOUNT,
} from "../util/constants";
import { describeDevMoonbeam } from "../util/setup-dev-tests";

describeDevMoonbeam("Democracy - genesis and preimage", (context) => {
  let genesisAccount: KeyringPair;

  before("Setup genesis account for substrate", async () => {
    const keyring = new Keyring({ type: "ethereum" });
    genesisAccount = await keyring.addFromUri(GENESIS_ACCOUNT_PRIVATE_KEY, null, "ethereum");
  });
  it("should check initial state", async function () {
    // referendumCount
    const referendumCount = await context.polkadotApi.query.democracy.referendumCount();
    expect(referendumCount.toHuman()).to.equal("0");
  });
  it("notePreimage", async function () {
    // notePreimage
    const proposal =
      context.polkadotApi.tx.parachainStaking.setParachainBondAccount(GENESIS_ACCOUNT);
    const encodedProposal = (proposal as SubmittableExtrinsic)?.method.toHex() || "";
    const encodedHash = blake2AsHex(encodedProposal);
    await context.polkadotApi.tx.democracy
      .notePreimage(encodedProposal)
      .signAndSend(genesisAccount);
    await context.createBlock();

    const preimageStatus = await context.polkadotApi.query.democracy.preimages(encodedHash);
    expect((preimageStatus.toHuman() as any).Available.provider).to.equal(GENESIS_ACCOUNT);
    expect((preimageStatus.toHuman() as any).Available.deposit).to.equal("2.2000 mUNIT");
  });
});

describeDevMoonbeam("Democracy - propose", (context) => {
  let genesisAccount: KeyringPair;
  let encodedHash: string;

  before("Setup genesis account for substrate", async () => {
    const keyring = new Keyring({ type: "ethereum" });
    genesisAccount = await keyring.addFromUri(GENESIS_ACCOUNT_PRIVATE_KEY, null, "ethereum");

    // notePreimage
    const proposal =
      context.polkadotApi.tx.parachainStaking.setParachainBondAccount(GENESIS_ACCOUNT);
    const encodedProposal = (proposal as SubmittableExtrinsic)?.method.toHex() || "";
    encodedHash = blake2AsHex(encodedProposal);
    await context.polkadotApi.tx.democracy
      .notePreimage(encodedProposal)
      .signAndSend(genesisAccount);
    await context.createBlock();
  });
  it("propose", async function () {
    // propose
    await context.polkadotApi.tx.democracy
      .propose(encodedHash, PROPOSAL_AMOUNT)
      .signAndSend(genesisAccount);
    await context.createBlock();

    // referendumCount
    const referendumCount = await context.polkadotApi.query.democracy.referendumCount();
    expect(referendumCount.toHuman()).to.equal("0");

    // publicPropCount
    const publicPropCount = await context.polkadotApi.query.democracy.publicPropCount();
    expect(publicPropCount.toHuman()).to.equal("1");

    // publicProps
    const publicProps = await context.polkadotApi.query.democracy.publicProps();
    // encodedHash
    expect((publicProps.toHuman() as any)[0][1]).to.equal(encodedHash);
    // prop author
    expect((publicProps.toHuman() as any)[0][2]).to.equal(GENESIS_ACCOUNT);
    // depositOf
    const depositOf = await context.polkadotApi.query.democracy.depositOf(0);
    expect((depositOf.toHuman() as any)[1]).to.equal("1.0000 kUNIT");
  });
});

describeDevMoonbeam("Democracy - second proposal", (context) => {
  let genesisAccount: KeyringPair, alith: KeyringPair;
  let encodedHash: string;

  before("Setup genesis account for substrate", async () => {
    const keyring = new Keyring({ type: "ethereum" });
    genesisAccount = await keyring.addFromUri(GENESIS_ACCOUNT_PRIVATE_KEY, null, "ethereum");
    alith = await keyring.addFromUri(ALITH_PRIV_KEY, null, "ethereum");

    // notePreimage
    const proposal =
      context.polkadotApi.tx.parachainStaking.setParachainBondAccount(GENESIS_ACCOUNT);
    const encodedProposal = (proposal as SubmittableExtrinsic)?.method.toHex() || "";
    encodedHash = blake2AsHex(encodedProposal);
    await context.polkadotApi.tx.democracy
      .notePreimage(encodedProposal)
      .signAndSend(genesisAccount);
    await context.createBlock();
    // propose
    await context.polkadotApi.tx.democracy
      .propose(encodedHash, PROPOSAL_AMOUNT)
      .signAndSend(genesisAccount);
    await context.createBlock();
  });
  it("second proposal", async function () {
    // second
    await context.polkadotApi.tx.democracy.second(0, 1000).signAndSend(alith);
    await context.createBlock();

    // publicProps
    const publicProps = await context.polkadotApi.query.democracy.publicProps();
    // encodedHash
    expect((publicProps.toHuman() as any)[0][1]).to.equal(encodedHash);
    // prop author
    expect((publicProps.toHuman() as any)[0][2]).to.equal(GENESIS_ACCOUNT);

    // depositOf
    const depositOf = await context.polkadotApi.query.democracy.depositOf(0);
    expect((depositOf.toHuman() as any)[1]).to.equal("1.0000 kUNIT");
    expect((depositOf.toHuman() as any)[0][1]).to.equal(ALITH);
  });
});
describeDevMoonbeam("Democracy - vote on referendum", (context) => {
  let genesisAccount: KeyringPair, alith: KeyringPair;
  let encodedHash: string;
  let launchPeriod;

  before("Setup genesis account for substrate", async () => {
    const keyring = new Keyring({ type: "ethereum" });
    genesisAccount = await keyring.addFromUri(GENESIS_ACCOUNT_PRIVATE_KEY, null, "ethereum");
    alith = await keyring.addFromUri(ALITH_PRIV_KEY, null, "ethereum");

    // notePreimage
    const proposal =
      context.polkadotApi.tx.parachainStaking.setParachainBondAccount(GENESIS_ACCOUNT);
    const encodedProposal = (proposal as SubmittableExtrinsic)?.method.toHex() || "";
    encodedHash = blake2AsHex(encodedProposal);
    await context.polkadotApi.tx.democracy
      .notePreimage(encodedProposal)
      .signAndSend(genesisAccount);
    await context.createBlock();
    // propose
    await context.polkadotApi.tx.democracy
      .propose(encodedHash, PROPOSAL_AMOUNT)
      .signAndSend(genesisAccount);
    await context.createBlock();
    // second
    await context.polkadotApi.tx.democracy.second(0, 1000).signAndSend(alith);
    await context.createBlock();
  });
  it("check launch period", async function () {
    // launchPeriod
    launchPeriod = await context.polkadotApi.consts.democracy.launchPeriod;
    expect(launchPeriod.toHuman()).to.equal("7,200");
  });
  it("check referendum", async function () {
    this.timeout(20000);
    // let Launchperiod elapse to turn the proposal into a referendum
    // launchPeriod minus the 3 blocks that already elapsed
    for (let i = 0; i < Number(launchPeriod) - 3; i++) {
      await context.createBlock();
    }
    // referendumCount
    let referendumCount = await context.polkadotApi.query.democracy.referendumCount();
    expect(referendumCount.toHuman()).to.equal("1");

    // publicPropCount
    const publicPropCount = await context.polkadotApi.query.democracy.publicPropCount();
    expect(publicPropCount.toHuman()).to.equal("1");

    // referendumInfoOf
    const referendumInfoOf = await context.polkadotApi.query.democracy.referendumInfoOf(0);
    expect((referendumInfoOf.toHuman() as any).Ongoing.proposalHash).to.equal(encodedHash);
  });
  it("vote", async function () {
    // vote
    await context.polkadotApi.tx.democracy
      .vote(0, {
        Standard: { balance: VOTE_AMOUNT, vote: { aye: true, conviction: 1 } },
      })
      .signAndSend(alith);
    await context.createBlock();

    // referendumInfoOf
    const referendumInfoOf = await context.polkadotApi.query.democracy.referendumInfoOf(0);
    expect((referendumInfoOf.toHuman() as any).Ongoing.proposalHash).to.equal(encodedHash);
    expect((referendumInfoOf.toHuman() as any).Ongoing.tally.ayes).to.equal("10.0000 UNIT");
    expect((referendumInfoOf.toHuman() as any).Ongoing.tally.turnout).to.equal("10.0000 UNIT");
  });
});

// TODO verify that vote is enacted
