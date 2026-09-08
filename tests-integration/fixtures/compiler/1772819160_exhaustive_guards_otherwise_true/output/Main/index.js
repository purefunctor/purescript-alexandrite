import * as Data_Boolean from "../Data.Boolean/index.js";
import * as $foreign from "./foreign.js";
export function test(x) {
  {
    const n = x;
    if (lessThan(n)(0 | 0)) {
      return 0 | 0;
    }
    if (Data_Boolean.otherwise) {
      return n;
    }
  }
  throw new Error("Pattern match failure");
}
function test_(x) {
  {
    const n = x;
    if (lessThan(n)(0 | 0)) {
      return 0 | 0;
    }
    if (Data_Boolean.otherwise) {
      return n;
    }
  }
  throw new Error("Pattern match failure");
}
export function test2(x) {
  {
    const n = x;
    if (lessThan(n)(0 | 0)) {
      return 0 | 0;
    }
    if (true) {
      return n;
    }
  }
  throw new Error("Pattern match failure");
}
function test2_(x) {
  {
    const n = x;
    if (lessThan(n)(0 | 0)) {
      return 0 | 0;
    }
    if (true) {
      return n;
    }
  }
  throw new Error("Pattern match failure");
}
export const lessThan = $foreign["lessThan"];
export { test_ as "test'" };
export { test2_ as "test2'" };
