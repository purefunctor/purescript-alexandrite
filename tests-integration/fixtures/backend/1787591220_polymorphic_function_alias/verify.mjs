import * as Main from "./output/Main/index.js";

const value = Main.use;
if (value[1] !== 42 || value[2] !== "x") {
  throw new Error("expected the non-member alias to remain polymorphic");
}
