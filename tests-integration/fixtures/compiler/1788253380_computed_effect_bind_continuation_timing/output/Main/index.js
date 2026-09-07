import * as Control_Bind from "../Control.Bind/index.js";
import * as Data_Unit from "../Data.Unit/index.js";
import * as Effect from "../Effect/index.js";
import * as $foreign from "./foreign.js";
export function computedBind($unit) {
  const $function = Control_Bind.bind(Effect.bindEffect);
  const $value = Data_Unit.unit;
  const $effect = () => {
    return $value;
  };
  return /* @__PURE__ */ $function($effect)(makeContinuation(Data_Unit.unit));
}
export const makeContinuation = $foreign["makeContinuation"];
