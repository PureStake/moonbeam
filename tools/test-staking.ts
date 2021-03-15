import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { start } from "polkadot-launch";
import { typesBundle } from "../moonbeam-types-bundle";
import {
  ALITH,
  GERALD,
  FAITH,
  STAKING_AMOUNT,
  ETHAN_PRIVKEY,
  ETHAN,
  GENESIS_ACCOUNT_BALANCE,
  DEFAULT_GENESIS_BALANCE,
  ALITH_PRIVKEY,
  GLMR,
  MIN_GLMR_NOMINATOR,
  MIN_GLMR_STAKING,
} from "./test-constants";

function assert(condition: boolean, msg: string) {
  if (!condition) throw new Error(msg);
}

async function wait(duration: number) {
  console.log(`Waiting ${duration / 1000} seconds`);
  return new Promise((res) => {
    setTimeout(res, duration);
  });
}

async function test() {
  await start("config_moonbeam_staking.json");
  console.log("done");
  const WS_PORT = 36946;
  const wsProviderUrl = `ws://localhost:${WS_PORT}`;

  const wsProvider = new WsProvider(wsProviderUrl);
  const polkadotApi = await ApiPromise.create({
    provider: wsProvider,
    typesBundle: typesBundle as any,
  });

  // subscribe to all new headers (with extended info)
  let lastBlock = Date.now();
  polkadotApi.derive.chain.subscribeNewHeads((header) => {
    console.log(
      `New Block: #${header.number}: ${header.author}, time since last block: ${
        (Date.now() - lastBlock) / 1000
      } sec`
    );
    lastBlock = Date.now();
  });

  // Balance
  const account = await polkadotApi.query.system.account(ETHAN);
  // assert(
  //   account.data.free.toString() === DEFAULT_GENESIS_BALANCE.toString(),
  //   "wrong balance for Ethan, dif: "+(Number(DEFAULT_GENESIS_BALANCE)-Number(account.data.free))
  // );

  // Nominators
  const nominators = await polkadotApi.query.stake.nominators(GERALD);
  assert(nominators.toHuman() === null, "there should be no nominator");

  // Validators
  const validators = await polkadotApi.query.stake.validators();
  assert(validators.toHuman()[0].toLowerCase() === GERALD, "Gerald is not a validator");
  assert(validators.toHuman()[1].toLowerCase() === FAITH.toLowerCase(), "Faith is not a validator");

  // Candidates
  const candidates = await polkadotApi.query.stake.candidatePool();
  console.log("candidates", candidates.toHuman());
  assert(candidates.toHuman()[0].owner.toLowerCase() === GERALD, "Gerald is not a candidates");
  assert(
    candidates.toHuman()[1].owner.toLowerCase() === FAITH.toLowerCase(),
    "Faith is not a candidates"
  );
  assert(candidates.toHuman()[0].amount === STAKING_AMOUNT, "Gerald has wrong staking amount");
  assert(candidates.toHuman()[1].amount === STAKING_AMOUNT, "Faith has wrong staking amount");

  // Join Candidates
  const keyring = new Keyring({ type: "ethereum" });
  const testAccount = await keyring.addFromUri(ETHAN_PRIVKEY, null, "ethereum");
  const unsub = await polkadotApi.tx.stake
    .joinCandidates(0, MIN_GLMR_STAKING)
    .signAndSend(testAccount, ({ events = [], status }) => {
      console.log(`Current status is ${status.type}`);

      if (status.isFinalized) {
        console.log(`Transaction finalized at blockHash ${status.asFinalized}`);

        // Loopcod through Vec<EventRecord> to display all events
        events.forEach(({ phase, event: { data, method, section } }) => {
          console.log(`\t' ${phase}: ${section}.${method}:: ${data}`);
        });

        unsub();
      }
    });
  await wait(50000);
  const candidatesAfter = await polkadotApi.query.stake.candidatePool();
  console.log("candidatesAfter", candidatesAfter.toHuman());
  assert(
    (candidatesAfter.toHuman() as { owner: string; amount: string }[]).length === 3,
    "new candidate should have been added"
  );
  const ethan = await polkadotApi.query.system.account(ETHAN);
  console.log(ethan.data.free.toString());
  console.log(DEFAULT_GENESIS_BALANCE.toString());

  // Join Nominators
  const keyringAlith = new Keyring({ type: "ethereum" });
  const alith = await keyringAlith.addFromUri(ALITH_PRIVKEY, null, "ethereum");
  const unsub2 = await polkadotApi.tx.stake
    .joinNominators(GERALD, MIN_GLMR_NOMINATOR)
    .signAndSend(alith, ({ events = [], status }) => {
      console.log(`Current status is ${status.type}`);

      if (status.isFinalized) {
        console.log(`Transaction finalized at blockHash ${status.asFinalized}`);

        // Loop through Vec<EventRecord> to display all events
        events.forEach(({ phase, event: { data, method, section } }) => {
          console.log(`\t' ${phase}: ${section}.${method}:: ${data}`);
        });

        unsub2();
      }
    });
  await wait(60000);
  const nominatorsAfter = await polkadotApi.query.stake.nominators(ALITH);
  assert(
    (nominatorsAfter.toHuman() as {
      nominations: { owner: string; amount: string }[];
    }).nominations[0].owner.toLowerCase() === GERALD,
    "nomination didnt go through"
  );
  console.log("nominatorsAfter", nominatorsAfter.toHuman());

  // Revoke Nomination
  const unsub3 = await polkadotApi.tx.stake
    .revokeNomination(GERALD)
    .signAndSend(alith, ({ events = [], status }) => {
      console.log(`Current status is ${status.type}`);

      if (status.isFinalized) {
        console.log(`Transaction finalized at blockHash ${status.asFinalized}`);

        // Loop through Vec<EventRecord> to display all events
        events.forEach(({ phase, event: { data, method, section } }) => {
          console.log(`\t' ${phase}: ${section}.${method}:: ${data}`);
        });

        unsub3();
      }
    });
  await wait(60000);
  const nominatorsAfterRevocation = await polkadotApi.query.stake.nominators(ALITH);
  console.log("nominatorsAfterRevocation", nominatorsAfterRevocation.toHuman());

  console.log("SUCCESS");
}
test();

// check author for blocks
// query author-inherent.author in storage in
// https://polkadot.js.org/docs/api/start/api.query.other at every block

// {candidate/nominator}_bond_more/less for validators/nominators
// revoke_nomination
// leave_candidates

// wait to test adding more than one nomination for a nominator
