import * as Control_Bind from "../Control.Bind/index.js";
import * as Effect from "../Effect/index.js";
import * as $foreign from "./foreign.js";

export function genericBind(bindMDict) {
  return Control_Bind.bind(bindMDict);
}

export function aliased(seed) {
  return genericBind(Effect.bindEffect)(constructEffect("alias-first")(seed))(
    value => constructEffect("alias-second")(value)
  );
}

export const constructEffect = $foreign["constructEffect"];
export const mark = $foreign["mark"];

export const deferredEffect = (() => {
  const $action = constructEffect("deferred-action")("ignored");
  return () => {
    const value = $action();
    return constructEffect("deferred-result")(deferredValue)();
  };
})();

export const deferredValue = mark("deferred-value")("deferred");
