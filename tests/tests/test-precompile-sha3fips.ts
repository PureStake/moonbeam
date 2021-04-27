import { expect } from "chai";
import { describeDevMoonbeam } from "../util/setup-dev-tests";
import { customWeb3Request } from "../util/providers";
import { GENESIS_ACCOUNT } from "../util/constants";

describeDevMoonbeam("Precompiles - sha3fips", (context) => {
  // Test taken from https://github.com/binance-chain/bsc/pull/118
  it("sha3fips should be valid", async function () {
    const tx_call = await customWeb3Request(context.web3, "eth_call", [
      {
        from: GENESIS_ACCOUNT,
        value: "0x0",
        gas: "0x10000",
        gasPrice: "0x01",
        to: "0x0000000000000000000000000000010000000001",
        data:
          "0x0448250ebe88d77e0a12bcf530fe6a2cf1ac176945638d309b840d631940c93b78c2bd6d16f227a8877e" +
          "3f1604cd75b9c5a8ab0cac95174a8a0a0f8ea9e4c10bca",
      },
    ]);
    expect(tx_call.result).equals(
      "0xc7647f7e251bf1bd70863c8693e93a4e77dd0c9a689073e987d51254317dc704"
    );
  });
});
