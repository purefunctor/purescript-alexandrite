import * as $foreign from "./foreign.js";
function $const(a) {
  return ($b) => {
    return a;
  };
}
export const add = $foreign["add"];
export { $const as "const" };
