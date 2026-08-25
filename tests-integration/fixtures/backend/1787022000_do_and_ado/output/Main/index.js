import * as Control_Applicative from "../Control.Applicative/index.js";
import * as Control_Apply from "../Control.Apply/index.js";
import * as Control_Bind from "../Control.Bind/index.js";
import * as Data_Functor from "../Data.Functor/index.js";
import * as Effect from "../Effect/index.js";
import * as $foreign from "./foreign.js";

export const firstAction = $foreign["firstAction"];
export const secondAction = $foreign["secondAction"];
export const independentAction = $foreign["independentAction"];

export const sequential = Control_Bind.bind(Effect.bindEffect1)(firstAction)(
  first => secondAction(first)
);

export const independent = Control_Apply.apply(Effect.applyEffect1)(
  Data_Functor.map(Effect.functorEffect)(first => second => ({ first: first, second: second }))(
    firstAction
  )
)(independentAction);

export const pureValue = Control_Applicative.pure(Effect.applicativeEffect)(42 | 0);
