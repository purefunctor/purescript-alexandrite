import * as Control_Applicative from "../Control.Applicative/index.js";
import * as Control_Apply from "../Control.Apply/index.js";
import * as Control_Monad_ST_Internal from "../Control.Monad.ST.Internal/index.js";
import * as Data_Functor from "../Data.Functor/index.js";
import * as $foreign from "./foreign.js";

export function timedAdo(seed) {
  return Control_Apply.apply(Control_Monad_ST_Internal.applyST)(
    Data_Functor.map(Control_Monad_ST_Internal.functorST)(
      first => second => ({ first: first, second: second })
    )(constructST("ado-first")(seed))
  )(constructST("ado-second")({ seed: seed }));
}

export function identity(value) {
  return value;
}

export function mapped(value) {
  return Data_Functor.map(Control_Monad_ST_Internal.functorST)(mark("map-function")(identity))(
    constructST("map-action")(value)
  );
}

export function applied(value) {
  return Control_Apply.apply(Control_Monad_ST_Internal.applyST)(
    constructST("apply-function-action")(identity)
  )(constructST("apply-value-action")(value));
}

export function capturedPure(value) {
  return Control_Applicative.pure(Control_Monad_ST_Internal.applicativeST)(
    mark("pure-value")(value)
  );
}

export const constructST = $foreign["constructST"];
export const mark = $foreign["mark"];
