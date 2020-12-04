import { mnemonicToSeedSync, generateMnemonic } from "bip39";
import { hdkey } from "ethereumjs-wallet";
import * as yargs from "yargs";

const argv = (yargs as any)(process.argv.slice(2))
  .usage('Usage: $0 [--mnemonic "..."] [--account-index x]')
  .version("1.0.0")
  .options({
    mnemonic: { type: "string" },
    "account-index": { type: "number", default: 0 },
  }).argv;

const account_index = argv["account-index"];
const mnemonic = argv["mnemonic"] || generateMnemonic();

const main = async () => {
  const hdwallet = hdkey.fromMasterSeed(mnemonicToSeedSync(mnemonic));
  const path = `m/44'/60'/0'/0/${account_index}`;
  const wallet = hdwallet.derivePath(path).getWallet();

  console.log(`Address:      ${wallet.getAddressString()}`);
  console.log(`Mnemonic:     ${mnemonic}`);
  console.log(`Private Key:  ${wallet.getPrivateKeyString()}`);
  console.log(`Path:         ${path}`);
};

main();
