import * as Control_Bind from "../Control.Bind/index.js";
import * as Control_Monad_ST_Internal from "../Control.Monad.ST.Internal/index.js";
import * as $foreign from "./foreign.js";

export function branched(choose) {
  return seed => {
    const $function = Control_Bind.bind(Control_Monad_ST_Internal.bindST)(
      constructST("branch-action")(seed)
    );
    const $closure = value => {
      if (choose) {
        return constructST("branch-then")(value);
      } else {
        return constructST("branch-else")(value);
      }
    };
    const $argument = $closure;
    const $call = $function($argument);
    return $call;
  };
}

export function patternLet(seed) {
  const $function = Control_Bind.bind(Control_Monad_ST_Internal.bindST)(
    constructST("pattern-action")(seed)
  );
  const $closure = value => {
    const $scrutinee = { selected: value };
    const selected = $scrutinee.selected;
    return constructST("pattern-result")(selected);
  };
  const $argument = $closure;
  const $call = $function($argument);
  return $call;
}

export function genericBind(bindMDict) {
  return Control_Bind.bind(bindMDict);
}

export function aliased(seed) {
  return genericBind(Control_Monad_ST_Internal.bindST)(constructST("alias-first")(seed))(
    value => constructST("alias-second")(value)
  );
}

export const constructST = $foreign["constructST"];
export const mark = $foreign["mark"];

export const deferredST = Control_Bind.bind(Control_Monad_ST_Internal.bindST)(
  constructST("deferred-action")("ignored")
)(value => constructST("deferred-result")(deferredValue));

export const deferredValue = mark("deferred-value")("deferred");
