// GitHub creates and signs the commit from the staged file contents.
import { execFileSync } from "node:child_process";

export function githubAppApi(root, token, args, input) {
  try {
    return execFileSync("gh", ["api", ...args], {
      cwd: root,
      env: { ...process.env, GH_TOKEN: token },
      encoding: "utf8",
      input,
      stdio: [input === undefined ? "ignore" : "pipe", "pipe", "pipe"],
    }).trim();
  } catch {
    throw new Error(
      `GitHub App API failed: ${args[0] === "graphql" ? "create signed commit" : args.slice(0, 3).join(" ")}`
    );
  }
}

export function createSignedCommit(
  root,
  repository,
  branchName,
  start,
  title,
  token
) {
  const staged = execFileSync(
    "git",
    ["diff", "--cached", "--name-only", "-z"],
    {
      cwd: root,
      encoding: "utf8",
    }
  );
  const additions = staged
    .split("\0")
    .filter(Boolean)
    .map((path) => ({
      path,
      contents: execFileSync("git", ["show", `:${path}`], {
        cwd: root,
      }).toString("base64"),
    }));
  const response = JSON.parse(
    githubAppApi(
      root,
      token,
      ["graphql", "--method", "POST", "--input", "-"],
      JSON.stringify({
        query: `mutation ($input: CreateCommitOnBranchInput!) {
        createCommitOnBranch(input: $input) { commit { oid } }
      }`,
        variables: {
          input: {
            branch: { repositoryNameWithOwner: repository, branchName },
            expectedHeadOid: start,
            message: { headline: title },
            fileChanges: { additions },
          },
        },
      })
    )
  );
  const oid = response.data?.createCommitOnBranch?.commit?.oid;
  if (response.errors?.length || !/^[0-9a-f]{40}$/.test(oid || ""))
    throw new Error("GitHub did not return a commit ID");
  return oid;
}
