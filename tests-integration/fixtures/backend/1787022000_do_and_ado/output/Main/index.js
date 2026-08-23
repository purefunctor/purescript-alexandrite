import * as Effect from "../Effect/index.js";
import * as $foreign from "./foreign.js";

export const firstAction = $foreign["firstAction"];
export const secondAction = $foreign["secondAction"];
export const independentAction = $foreign["independentAction"];

export const sequential = (() => {
  return Effect.bindEffect1.bind(firstAction)(first => secondAction(first));
})();

export const independent = (() => {
  return Effect.applyEffect1.apply(
    Effect.functorEffect.map(first => second => ({ first: first, second: second }))(firstAction)
  )(independentAction);
})();

export const pureValue = Effect.applicativeEffect.pure(42 | 0);
