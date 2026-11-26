import anbani from "../src/anbani.mjs";

const [cmd, ...rest] = process.argv.slice(2);

const die = (err) => {
  console.error(`error: ${err.message ?? err}`);
  process.exit(1);
};

const commands = {
  convert([text, from, to]) {
    console.log(anbani.core.convert(text, from, to));
  },
  interpret([text, to = "mtavruli"]) {
    console.log(anbani.core.interpret(text, to));
  },
  lorem([nwords = "8"]) {
    console.log(anbani.lorem.sentences(parseInt(nwords, 10)));
  },
  georgianise([text, mode = "balanced"]) {
    console.log(anbani.nlp.georgianisation.georgianise(text, mode));
  },
  latinise([text]) {
    console.log(anbani.nlp.georgianisation.latinise(text));
  },
  expand([text]) {
    console.log(anbani.nlp.contractions.expandText(text));
  },
  contract([text]) {
    console.log(anbani.nlp.contractions.contractText(text));
  },
  async tui() {
    if (process.stdin.isTTY && process.stdout.isTTY) {
      const { run } = await import("../src/tui/app.mjs");
      process.exit(await run());
    }
    console.error("error: anbani tui requires an interactive terminal");
    process.exit(2);
  },
  help() {
    console.log(
      "Usage: anbani <command> ...\n" +
        "  convert     <text> <from> <to>\n" +
        "  interpret   <text> [to=mtavruli]\n" +
        "  georgianise <text> [mode=balanced|fast]\n" +
        "  latinise    <text>\n" +
        "  expand      <text>\n" +
        "  contract    <text>\n" +
        "  lorem       [words=8]\n" +
        "  tui                      (interactive UI)\n" +
        "  <text> [to]              (shorthand for: interpret <text> [to])"
    );
  },
};

if (!cmd) {
  if (process.stdin.isTTY && process.stdout.isTTY) {
    const { run } = await import("../src/tui/app.mjs");
    process.exit(await run());
  }
  commands.help();
} else if (Object.prototype.hasOwnProperty.call(commands, cmd)) {
  try {
    commands[cmd](rest);
  } catch (err) {
    die(err);
  }
} else {
  try {
    console.log(anbani.core.interpret(cmd, rest[0] ?? "mtavruli"));
  } catch (err) {
    die(err);
  }
}
