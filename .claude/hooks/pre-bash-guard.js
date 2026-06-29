#!/usr/bin/env node
// PreToolUse(Bash) guard for better-auth-rs.
// Steers the agent to the project's build/test discipline by denying a few wrong commands.
// Reads the tool-call JSON from stdin; emits a permissionDecision:"deny" object to block.

function read() {
  return new Promise((resolve) => {
    let buf = "";
    process.stdin.setEncoding("utf8");
    process.stdin.on("data", (c) => (buf += c));
    process.stdin.on("end", () => resolve(buf));
  });
}

function deny(reason) {
  process.stdout.write(
    JSON.stringify({
      hookSpecificOutput: {
        hookEventName: "PreToolUse",
        permissionDecision: "deny",
        permissionDecisionReason: reason,
      },
    }),
  );
  process.exit(0);
}

(async () => {
  let input;
  try {
    input = JSON.parse((await read()) || "{}");
  } catch {
    process.exit(0);
  }
  if (input.tool_name !== "Bash") process.exit(0);
  const command = (input.tool_input && input.tool_input.command) || "";
  if (!command) process.exit(0);

  // Tokenize, respecting simple quotes; strip leading inline ENV=val assignments.
  let tokens =
    command.match(/(?:[^\s"']+|"[^"]*"|'[^']*')+/g)?.map((t) => t.replace(/^['"]|['"]$/g, "")) || [];
  while (tokens.length && /^[A-Za-z_][A-Za-z0-9_]*=/.test(tokens[0]) && !tokens[0].includes("/")) {
    tokens = tokens.slice(1);
  }
  if (!tokens.length) process.exit(0);

  const base = (p) => p.split("/").pop();
  const argv0 = base(tokens[0]);
  const rest = tokens.slice(1);
  const positional = rest.filter((a) => !a.startsWith("-"));

  // 1) Never call rustfmt directly — it ignores the workspace edition; CI checks `cargo fmt`.
  if (argv0 === "rustfmt") {
    deny("error: Use `cargo fmt --all` instead of `rustfmt` directly — it respects the workspace edition (2024) and matches what CI checks.");
  }

  // 2) Don't run the whole test suite from the repo root without a -p/filter/path.
  if (argv0 === "cargo") {
    const sub = positional[0];
    const isTest = sub === "test" || (sub === "nextest" && positional[1] === "run");
    if (isTest) {
      const hasScope =
        rest.some((a) => a === "-p" || a === "--package" || a.startsWith("--package=")) ||
        rest.includes("--workspace") === false &&
          positional.slice(sub === "nextest" ? 2 : 1).length > 0; // a filter/path/test-name
      const wantsAll = rest.includes("--workspace");
      if (!hasScope && !wantsAll) {
        deny("error: Scope your tests: `cargo nextest run -p <crate> [filter]`. Running the full suite from the repo root is slow; pass -p/--package or a filter (or --workspace if you really mean everything).");
      }
    }
  }

  process.exit(0);
})();
