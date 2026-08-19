import { readFile } from "node:fs/promises";
import * as Direct from "./output/Direct/index.js";
import * as Main from "./output/Main/index.js";
import * as Origin from "./output/Origin/index.js";
import * as Transitive from "./output/Transitive/index.js";

const directSource = await readFile(new URL("./output/Direct/index.js", import.meta.url), "utf8");
const originSource = await readFile(new URL("./output/Origin/index.js", import.meta.url), "utf8");
const transitiveSource = await readFile(
  new URL("./output/Transitive/index.js", import.meta.url),
  "utf8",
);

if (directSource.includes("import * as") || transitiveSource.includes("import * as")) {
  throw new Error("pure re-export modules contain namespace imports");
}
if (
  !directSource.includes(
    'export { Just, "await", foreignValue, visible } from "../Origin/index.js";',
  ) ||
  !transitiveSource.includes('export { append } from "../Direct/index.js";') ||
  !transitiveSource.includes(
    'export { Just, "await", foreignValue, visible } from "../Origin/index.js";',
  )
) {
  throw new Error("grouped module re-exports are missing");
}
if (
  originSource.includes('"<>"') ||
  directSource.includes('"<>"') ||
  transitiveSource.includes('"<>"')
) {
  throw new Error("source-level operator leaked into the JavaScript ABI");
}

const assertKeys = (namespace, expected, name) => {
  const actual = Object.keys(namespace).sort();
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(`${name} exports ${JSON.stringify(actual)}`);
  }
};

assertKeys(
  Origin,
  ["Just", "append", "await", "eqOption", "foreignValue", "measureInt", "visible"],
  "Origin",
);
assertKeys(Direct, ["Just", "append", "await", "foreignValue", "visible"], "Direct");
assertKeys(
  Transitive,
  ["Just", "append", "await", "foreignValue", "marker", "visible"],
  "Transitive",
);
assertKeys(
  Main,
  [
    "Just",
    "append",
    "await",
    "constructorValue",
    "foreignResult",
    "foreignValue",
    "hostileResult",
    "localCollision",
    "marker",
    "measured",
    "operatorValue",
    "transitiveMarker",
    "visible",
  ],
  "Main",
);

if (Direct.Just !== Origin.Just || Transitive.Just !== Origin.Just || Main.Just !== Origin.Just) {
  throw new Error("constructor re-export identity");
}
if (JSON.stringify(Transitive.Just(42)) !== JSON.stringify(["Just", 42])) {
  throw new Error("constructor representation");
}
if (
  Direct.visible !== Origin.visible ||
  Transitive.visible !== Origin.visible ||
  Main.visible !== Origin.visible
) {
  throw new Error("function re-export identity");
}
if (
  Direct.append === Origin.append ||
  Transitive.append !== Direct.append ||
  Main.append !== Direct.append
) {
  throw new Error("local collision");
}
if (
  Direct.foreignValue !== Origin.foreignValue ||
  Transitive.await !== Origin.await ||
  Main.foreignValue !== Origin.foreignValue ||
  Main.await !== Origin.await
) {
  throw new Error("foreign or hostile re-export identity");
}
if (Main.marker !== Transitive.marker) {
  throw new Error("transitive re-export identity");
}

const actual = {
  constructorValue: Main.constructorValue,
  operatorValue: Main.operatorValue,
  localCollision: Main.localCollision,
  foreignResult: Main.foreignResult,
  hostileResult: Main.hostileResult,
  measured: Main.measured,
  transitiveMarker: Main.transitiveMarker,
};
const expected = {
  constructorValue: ["Just", 42],
  operatorValue: 23,
  localCollision: 99,
  foreignResult: 7,
  hostileResult: 17,
  measured: 41,
  transitiveMarker: 1,
};
if (JSON.stringify(actual) !== JSON.stringify(expected)) {
  throw new Error(`unexpected output ${JSON.stringify(actual)}`);
}
