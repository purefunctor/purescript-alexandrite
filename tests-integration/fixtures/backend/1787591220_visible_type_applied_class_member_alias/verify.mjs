import * as Main from "./output/Main/index.js";

if (Main.use({ name: "visible" })({}) !== "visible") {
  throw new Error("expected the visibly applied class member to use its dictionary");
}
