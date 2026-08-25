import * as Control_Applicative from "../Control.Applicative/index.js";
import * as Control_Bind from "../Control.Bind/index.js";
import * as Data_Unit from "../Data.Unit/index.js";
import * as Effect from "../Effect/index.js";
import * as $foreign from "./foreign.js";

export function chained(seed) {
  return Control_Bind.bind(Effect.bindEffect)(constructEffect("first")(seed))(
    first => Control_Bind.bind(Effect.bindEffect)(constructEffect("second")({ first: first }))(
      second => constructEffect("third")({ first: first, second: second })
    )
  );
}

export function discarded(seed) {
  const $function = Control_Bind.discard(Control_Bind.discardUnit)(Effect.bindEffect)(
    constructEffect("discard-first")(Data_Unit.Unit)
  );
  const $closure = $unit => {
    const result = mark("discard-let")(seed);
    return constructEffect("discard-second")(result);
  };
  const $argument = $closure;
  const $call = $function($argument);
  return $call;
}

export function pureAfterBind(seed) {
  return Control_Bind.bind(Effect.bindEffect)(constructEffect("pure-action")(seed))(
    value => Control_Applicative.pure(Effect.applicativeEffect)(mark("pure-body")({ value: value }))
  );
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

export const deferredEffect = Control_Bind.bind(Effect.bindEffect)(
  constructEffect("deferred-action")("ignored")
)(value => constructEffect("deferred-result")(deferredValue));

export const deferredValue = mark("deferred-value")("deferred");
