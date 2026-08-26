import * as Main from "./output/Main/index.js";

if (Main.local[0] !== "Local" || Main.local[1] !== 1 || Main.local[2] !== "local") {
  throw new Error("expected the local constructor application to retain its representation");
}

const partial = Main.partial("partial");
if (partial[0] !== "Local" || partial[1] !== 2 || partial[2] !== "partial") {
  throw new Error("expected the partial constructor application to remain callable");
}

if (Main.empty !== "Empty") {
  throw new Error("expected the nullary constructor to retain its representation");
}

if (Main.external[0] !== "External" || Main.external[1] !== "external") {
  throw new Error("expected the external constructor application to retain its representation");
}
