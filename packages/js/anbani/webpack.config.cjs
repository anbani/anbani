const path = require("path");

const isProduction = process.env.NODE_ENV == "production";

const config = {
  entry: "./src/anbani.mjs",
  output: {
    path: path.resolve(__dirname, "dist"),
    filename: "anbani.js",
    globalObject: "this",
    library: {
      name: "anbani",
      type: "umd",
      export: "default",
      umdNamedDefine: true,
    }
  },
  resolve: {
    alias: {
      [path.resolve(__dirname, "src/lib/data-io.mjs")]: path.resolve(
        __dirname,
        "src/lib/data-io.browser.mjs"
      ),
    },
  },
  module: {
    rules: [
      { test: /\.csv$/, type: "asset/source" },
    ],
  },
};

module.exports = () => {
  if (isProduction) {
    config.mode = "production";
  } else {
    config.mode = "development";
  }
  return config;
};
