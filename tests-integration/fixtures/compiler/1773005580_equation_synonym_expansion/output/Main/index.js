import * as $foreign from "./foreign.js";
export function test(a) {
  return unsafeCoerce(a);
}
export const unsafeCoerce = $foreign["unsafeCoerce"];
