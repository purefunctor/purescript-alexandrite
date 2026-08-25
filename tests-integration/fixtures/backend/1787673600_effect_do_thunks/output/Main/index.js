import * as Control_Bind from "../Control.Bind/index.js";
import * as Data_Unit from "../Data.Unit/index.js";
import * as Effect from "../Effect/index.js";
import * as $foreign from "./foreign.js";

export function chained(seed) {
  const $action = constructEffect("first")(seed);
  const $effect = () => {
    const first = $action();
    const $action$1 = constructEffect("second")({ first: first });
    const second = $action$1();
    return constructEffect("third")({ first: first, second: second })();
  };
  return $effect;
}

export function discarded(seed) {
  const $action = constructEffect("discard-first")(Data_Unit.Unit);
  const $effect = () => {
    const $unit = $action();
    const result = mark("discard-let")(seed);
    return constructEffect("discard-second")(result)();
  };
  return $effect;
}

export function pureAfterBind(seed) {
  const $action = constructEffect("pure-action")(seed);
  const $effect = () => {
    const value = $action();
    const $value = mark("pure-body")({ value: value });
    return $value;
  };
  return $effect;
}

export function branched(choose) {
  return seed => {
    const $action = constructEffect("branch-action")(seed);
    const $effect = () => {
      const value = $action();
      if (choose) {
        return constructEffect("branch-then")(value)();
      } else {
        return constructEffect("branch-else")(value)();
      }
    };
    return $effect;
  };
}

export function patternLet(seed) {
  const $action = constructEffect("pattern-action")(seed);
  const $effect = () => {
    const value = $action();
    const $scrutinee = { selected: value };
    const selected = $scrutinee.selected;
    return constructEffect("pattern-result")(selected)();
  };
  return $effect;
}

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
  const $effect = () => {
    const value = $action();
    return constructEffect("deferred-result")(deferredValue)();
  };
  return $effect;
})();

export const deferredValue = mark("deferred-value")("deferred");
