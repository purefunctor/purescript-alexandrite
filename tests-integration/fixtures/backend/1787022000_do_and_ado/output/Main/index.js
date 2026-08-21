import * as Effect from "../Effect/index.js";
import * as $foreign from "./foreign.js";

function sequential$initialize$closure(first) {
  return secondAction(first);
}

function independent$initialize$closure(first) {
  return second => {
    return { first: first, second: second };
  };
}

export const firstAction = $foreign["firstAction"];
export const secondAction = $foreign["secondAction"];
export const independentAction = $foreign["independentAction"];

export const sequential = Effect.bindEffect1.bind(firstAction)(sequential$initialize$closure);

export const independent = Effect.applyEffect1.apply(
  Effect.functorEffect.map(independent$initialize$closure)(firstAction)
)(independentAction);

export const pureValue = Effect.applicativeEffect.pure(42 | 0);
