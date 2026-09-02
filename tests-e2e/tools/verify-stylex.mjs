import fs from "node:fs";
import path from "node:path";
import process from "node:process";

import { transformSync } from "@babel/core";
import stylexPlugin from "@stylexjs/babel-plugin";

const outputRoot = path.resolve(process.argv[2]);
const modules = ["Tokens", "Main"];
const styles = [];

for (const moduleName of modules) {
  const filename = path.join(outputRoot, moduleName, "index.js");
  const source = fs.readFileSync(filename, "utf8");
  const result = transformSync(source, {
    filename,
    babelrc: false,
    configFile: false,
    plugins: [
      [
        stylexPlugin,
        {
          dev: false,
          unstable_moduleResolution: {
            type: "commonJS",
            rootDir: outputRoot,
            themeFileExtension: "index",
          },
        },
      ],
    ],
  });

  if (result.code.includes("$stylex.")) {
    throw new Error(`${moduleName} retains uncompiled StyleX calls`);
  }
  styles.push(...result.metadata.stylex.map(([, style]) => style.ltr));
}

const css = styles.join("\n");
for (const expected of ["--", ":where(", "color:red"]) {
  if (!css.includes(expected)) {
    throw new Error(`StyleX metadata does not contain ${JSON.stringify(expected)}`);
  }
}
