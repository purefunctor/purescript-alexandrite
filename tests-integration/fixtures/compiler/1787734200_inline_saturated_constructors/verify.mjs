import * as Main from "./output/Main/index.js";

if (Main.local.tag !== "Local" || Main.local._1 !== 1 || Main.local._2 !== "local") {
  throw new Error("expected the local constructor application to retain its representation");
}

const partial = Main.partial("partial");
if (partial.tag !== "Local" || partial._1 !== 2 || partial._2 !== "partial") {
  throw new Error("expected the partial constructor application to remain callable");
}

if (Main.empty !== "Empty") {
  throw new Error("expected the nullary constructor to retain its representation");
}

if (Main.external.tag !== "External" || Main.external._1 !== "external") {
  throw new Error("expected the external constructor application to retain its representation");
}
