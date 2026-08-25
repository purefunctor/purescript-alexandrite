import * as Control_Applicative from "../Control.Applicative/index.js";
import * as Control_Apply from "../Control.Apply/index.js";
import * as Data_Functor from "../Data.Functor/index.js";
import * as Effect from "../Effect/index.js";
import * as $foreign from "./foreign.js";

export function timedAdo(seed) {
  return Control_Apply.apply(Effect.applyEffect)(
    Data_Functor.map(Effect.functorEffect)(first => second => ({ first: first, second: second }))(
      constructEffect("ado-first")(seed)
    )
  )(constructEffect("ado-second")({ seed: seed }));
}

export function identity(value) {
  return value;
}

export function mapped(value) {
  return Data_Functor.map(Effect.functorEffect)(mark("map-function")(identity))(
    constructEffect("map-action")(value)
  );
}

export function applied(value) {
  return Control_Apply.apply(Effect.applyEffect)(
    constructEffect("apply-function-action")(identity)
  )(constructEffect("apply-value-action")(value));
}

export function capturedPure(value) {
  return Control_Applicative.pure(Effect.applicativeEffect)(mark("pure-value")(value));
}

export const constructEffect = $foreign["constructEffect"];
export const mark = $foreign["mark"];
