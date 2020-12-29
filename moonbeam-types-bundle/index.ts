import type { OverrideBundleDefinition, OverrideBundleType } from "@polkadot/types/types";
export const moonbeamDefinitions = {
  types: [
    {
      minmax: [0, 4],
      types: {
        AccountId: "EthereumAccountId",
        Address: "AccountId",
        Balance: "u128",
        RefCount: "u8",
        LookupSource: "AccountId",
        Account: {
          nonce: "U256",
          balance: "u128",
        },
        ExitReason: {
          _enum: {
            ExitSucceed: "bool",
            ExitError: "bool",
            ExitRevert: "bool",
            ExitFatal: "bool",
          },
        },
      },
    },
    {
      minmax: [5, 5],
      types: {
        AccountId: "EthereumAccountId",
        Address: "AccountId",
        Balance: "u128",
        LookupSource: "AccountId",
        Account: {
          nonce: "U256",
          balance: "u128",
        },
        ExitReason: {
          _enum: {
            Succeed: "ExitSucceed",
            Error: "ExitError",
            Revert: "ExitRevert",
            verFatal: "ExitFatal",
          },
        },
        ExitSucceed: {
          _enum: ["Stopped", "Returned", "Suicided"],
        },
        ExitError: {
          _enum: [
            "StackUnderflow",
            "StackOverflow",
            "InvalidJump",
            "InvalidRange",
            "DesignatedInvalid",
            "CallTooDeep",
            "CreateCollision",
            "CreateContractLimit",
            "OutOfOffset",
            "OutOfGas",
            "OutOfFund",
            "PCUnderflow",
            "CreateEmpty",
            "Other(Cow<'static, str>)",
          ],
        },
        ExitRevert: {
          _enum: ["Reverted"],
        },
        ExitFatal: {
          _enum: [
            "NotSupported",
            "UnhandledInterrupt",
            "CallErrorAsFatal(ExitError)",
            "Other(Cow<'static, str>)",
          ],
        },
      },
    },
    {
      minmax: [6, undefined],
      types: {
        AccountId: "EthereumAccountId",
        Address: "AccountId",
        Balance: "u128",
        LookupSource: "AccountId",
        Account: {
          nonce: "U256",
          balance: "u128",
        },
        ExtrinsicSignature: "EthereumSignature",
        ExitReason: {
          _enum: {
            Succeed: "ExitSucceed",
            Error: "ExitError",
            Revert: "ExitRevert",
            verFatal: "ExitFatal",
          },
        },
        ExitSucceed: {
          _enum: ["Stopped", "Returned", "Suicided"],
        },
        ExitError: {
          _enum: [
            "StackUnderflow",
            "StackOverflow",
            "InvalidJump",
            "InvalidRange",
            "DesignatedInvalid",
            "CallTooDeep",
            "CreateCollision",
            "CreateContractLimit",
            "OutOfOffset",
            "OutOfGas",
            "OutOfFund",
            "PCUnderflow",
            "CreateEmpty",
            "Other(Cow<'static, str>)",
          ],
        },
        ExitRevert: {
          _enum: ["Reverted"],
        },
        ExitFatal: {
          _enum: [
            "NotSupported",
            "UnhandledInterrupt",
            "CallErrorAsFatal(ExitError)",
            "Other(Cow<'static, str>)",
          ],
        },
      },
    },
  ],
} as OverrideBundleDefinition;

export const typesBundle = {
  spec: {
    "moonbase-alphanet": moonbeamDefinitions,
    moonbeamDefinitions,
    "moonbeam-standalone": moonbeamDefinitions,
    "node-moonbeam": moonbeamDefinitions,
  },
} as OverrideBundleType;
