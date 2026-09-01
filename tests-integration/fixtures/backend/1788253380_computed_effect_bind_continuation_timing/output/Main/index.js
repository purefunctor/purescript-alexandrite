import * as Control_Bind from "../Control.Bind/index.js";
import * as Effect from "../Effect/index.js";
import * as $foreign from "./foreign.js";
export function computedBind($unit) {
  const $function = Control_Bind.bind(Effect.bindEffect1);
  const $effect = () => {
    return "Unit";
  };
  return /* @__PURE__ */ $function($effect)(makeContinuation("Unit"));
}
export const makeContinuation = $foreign["makeContinuation"];
