import * as Main from "./output/Main/index.js";

if (Main.use({ name: value => value })("x") !== "x") {
  throw new Error("expected the where-bound class member to use its dictionary");
}

if (Main.useLet({ name: value => value })("x") !== "x") {
  throw new Error("expected the let-bound class member to use its dictionary");
}
