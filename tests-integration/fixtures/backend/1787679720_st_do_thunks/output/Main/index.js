import * as Control_Applicative from "../Control.Applicative/index.js";
import * as Control_Bind from "../Control.Bind/index.js";
import * as Control_Monad_ST_Internal from "../Control.Monad.ST.Internal/index.js";
import * as Data_Unit from "../Data.Unit/index.js";
import * as $foreign from "./foreign.js";

export function chained(seed) {
  return Control_Bind.bind(Control_Monad_ST_Internal.bindST)(constructST("first")(seed))(
    first => Control_Bind.bind(Control_Monad_ST_Internal.bindST)(
      constructST("second")({ first: first })
    )(second => constructST("third")({ first: first, second: second }))
  );
}

export function discarded(seed) {
  const $function = Control_Bind.discard(Control_Bind.discardUnit)(
    Control_Monad_ST_Internal.bindST
  )(constructST("discard-first")(Data_Unit.Unit));
  const $closure = $unit => {
    const result = mark("discard-let")(seed);
    return constructST("discard-second")(result);
  };
  const $argument = $closure;
  const $call = $function($argument);
  return $call;
}

export function pureAfterBind(seed) {
  return Control_Bind.bind(Control_Monad_ST_Internal.bindST)(constructST("pure-action")(seed))(
    value => Control_Applicative.pure(Control_Monad_ST_Internal.applicativeST)(
      mark("pure-body")({ value: value })
    )
  );
}

export const constructST = $foreign["constructST"];
export const mark = $foreign["mark"];
