#!/usr/bin/env node

const { exec } = require("child_process");

const controller =
  typeof AbortController !== "undefined"
    ? new AbortController()
    : {
        abort: () => {},
        signal:
          typeof AbortSignal !== "undefined" ? new AbortSignal() : undefined,
      };
const { signal } = controller;

const command = ["prepare_swagger"]
  .concat(...process.argv.splice(2))
  .filter(Boolean)
  .join(" ");

exec(command, { signal }, (error, stdout, stderr) => {
  stdout && console.log(stdout);
  stderr && console.error(stderr);
  if (error !== null) {
    console.log(`exec error: ${error}`);
    process.exit(1);
  }
});

process.on("SIGTERM", () => {
  controller && controller.abort();
});

process.on("SIGINT", () => {
  controller && controller.abort();
});
