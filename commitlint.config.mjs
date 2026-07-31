export default {
  // The consolidated branch preserves merge commits from the reviewed feature
  // stack. Those historical subjects predate the repository's conventional
  // commit check, so leave them out while still linting new commits normally.
  ignores: [(commit) => /^(?:merge:|Merge )\s*PR #\d+\b/.test(commit)],
};
