import { execSync } from "child_process";
import yargs from "yargs";
import { getCompareLink } from "./github-utils";

function capitalize(s) {
  return s[0].toUpperCase() + s.slice(1);
}

const main = () => {
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

  const previousTag = argv.from;
  const newTag = argv.to;

  const moduleLinks = ["substrate", "polkadot", "cumulus", "frontier"].map((repoName) => ({
    name: repoName,
    link: getCompareLink(repoName, previousTag, newTag),
  }));
  const commits = execSync(`git log --oneline --pretty=format:"%s" ${previousTag}...${newTag}`)
    .toString()
    .split(`\n`)
    .filter((l) => !!l);

  const template = `
## Changes

${commits.map((commit) => `* ${commit}`).join("\n")}

## Dependency changes

Moonbeam: https://github.com/PureStake/moonbeam/compare/${previousTag}...${newTag}
${moduleLinks.map((modules) => `${capitalize(modules.name)}: ${modules.link}`).join("\n")}
`;
  console.log(template);
};

main();
