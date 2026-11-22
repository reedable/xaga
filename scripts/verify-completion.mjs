import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

// Keep the sequence inspectable and testable without running builds on import.
export const checks = [
  ["npm", ["run", "lint", "--", "--deny-warnings"]],
  ["npm", ["run", "format:check"]],
  ["npm", ["run", "typecheck"]],
  ["node", ["scripts/csp-hash.mjs", "--check"]],
  ["cargo", ["fmt", "--all", "--check"]],
  ["npm", ["run", "test:browser"]],
  ["npm", ["run", "check:rust"]],
  ["npm", ["run", "test:headless"]],
  ["npm", ["run", "test:gui"]],
];

export function verify(run = spawnSync) {
  for (const [command, args] of checks) {
    console.log(`\n> ${command} ${args.join(" ")}`);
    const result = run(command, args, {
      stdio: "inherit",
      env: {
        ...process.env,
        RUSTFLAGS: `${process.env.RUSTFLAGS ?? ""} -D warnings`.trim(),
        RUSTDOCFLAGS: `${process.env.RUSTDOCFLAGS ?? ""} -D warnings`.trim(),
      },
    });
    if (result.error) console.error(result.error.message);
    if (result.error || result.status !== 0) return result.status || 1;
  }
  return 0;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  process.exitCode = verify();
}
