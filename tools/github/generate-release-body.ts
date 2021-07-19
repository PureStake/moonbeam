import { Octokit } from "octokit";
import yargs from "yargs";
import { getCommitAndLabels, getCompareLink } from "./github-utils";

const BINARY_CHANGES_LABEL = "B5-clientnoteworthy";

function capitalize(s) {
  return s[0].toUpperCase() + s.slice(1);
}

async function main() {
  const argv = yargs(process.argv.slice(2))
    .usage("Usage: npm run ts-node github/generate-release-body.ts [args]")
    .version("1.0.0")
    .options({
      from: {
        type: "string",
        describe: "previous tag to retrieve commits from",
        required: true,
      },
      to: {
        type: "string",
        describe: "current tag being drafted",
        required: true,
      },
    })
    .demandOption(["from", "to"])
    .help().argv;

  const octokit = new Octokit({
    auth: process.env.GITHUB_TOKEN || undefined,
  });

  const previousTag = argv.from;
  const newTag = argv.to;

  const moduleLinks = ["substrate", "polkadot", "cumulus", "frontier"].map((repoName) => ({
    name: repoName,
    link: getCompareLink(repoName, previousTag, newTag),
  }));

  const { prByLabels } = await getCommitAndLabels(octokit, "PureStake", "moonbeam", previousTag, newTag);

  const template = `
## Changes

${prByLabels[BINARY_CHANGES_LABEL].map((pr) => `* ${pr.title} (#${pr.number})`).join("\n")}

## Dependency changes

Moonbeam: https://github.com/PureStake/moonbeam/compare/${previousTag}...${newTag}
${moduleLinks.map((modules) => `${capitalize(modules.name)}: ${modules.link}`).join("\n")}
`;
  console.log(template);
};

main();
