#!/usr/bin/env node
// PostToolUse(Write|Edit|MultiEdit) formatter for better-auth-rs.
// Format-only (no lint/fix): keeps edits tidy without changing semantics.
//  - *.rs    -> rustfmt --edition 2024
//  - *.toml  -> taplo fmt   (if installed)
// Anything else is left untouched. Failures are swallowed (never block an edit).

const { spawnSync } = require("node:child_process");

function read() {
  return new Promise((resolve) => {
    let buf = "";
    process.stdin.setEncoding("utf8");
    process.stdin.on("data", (c) => (buf += c));
    process.stdin.on("end", () => resolve(buf));
  });
}

function run(cmd, args) {
  try {
    spawnSync(cmd, args, { cwd: process.env.CLAUDE_PROJECT_DIR || process.cwd(), encoding: "utf8" });
  } catch {
    /* never block the edit */
  }
}

(async () => {
  let input;
  try {
    input = JSON.parse((await read()) || "{}");
  } catch {
    process.exit(0);
  }
  if (!["Write", "Edit", "MultiEdit"].includes(input.tool_name)) process.exit(0);
  const file = input.tool_input && input.tool_input.file_path;
  if (!file) process.exit(0);

  // Never touch the read-only vendored reference.
  if (file.includes("/reference/")) process.exit(0);

  if (file.endsWith(".rs")) {
    run("rustfmt", ["--edition", "2024", file]);
  } else if (file.endsWith(".toml")) {
    run("taplo", ["fmt", file]); // no-op if taplo isn't installed
  }
  process.exit(0);
})();
