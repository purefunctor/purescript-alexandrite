import * as $foreign from "./foreign.js";
export function localBind($unit) {
  const continuation = makeContinuation("Unit");
  return () => {
    return continuation("Unit")();
  };
}
export const makeContinuation = $foreign["makeContinuation"];
