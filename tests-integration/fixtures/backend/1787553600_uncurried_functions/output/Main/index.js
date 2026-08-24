import * as Data_Function_Uncurried from "../Data.Function.Uncurried/index.js";
import * as Lookalike from "../Lookalike/index.js";
import * as $foreign from "./foreign.js";

export function madeCaptured(captured) {
  return Data_Function_Uncurried.mkFn2(first => second => captured);
}

export function chooseSecondCurried(first) {
  return second => {
    return second;
  };
}

export const chooseSecond = $foreign["chooseSecond"];

export const made = (() => {
  return Data_Function_Uncurried.mkFn2(first => second => second);
})();

export const madeNested = (() => {
  function madeNested$initialize$closure(first) {
    return second => third => third;
  }
  return Data_Function_Uncurried.mkFn3(madeNested$initialize$closure);
})();

export const madeWithCurriedResult = (() => {
  return Data_Function_Uncurried.mkFn2(first => second => third => third);
})();

export const directRun = Data_Function_Uncurried.runFn2(chooseSecond)(1 | 0)(42 | 0);

export const directMadeRun = (() => {
  return Data_Function_Uncurried.runFn2(Data_Function_Uncurried.mkFn2(first => second => second))(
    1 | 0
  )(42 | 0);
})();

export const directNestedRun = Data_Function_Uncurried.runFn3(madeNested)(1 | 0)(2 | 0)(42 | 0);

export const directCapturedRun = Data_Function_Uncurried.runFn2(madeCaptured(42 | 0))(1 | 0)(2 | 0);

export const directCurriedResultRun = Data_Function_Uncurried.runFn2(madeWithCurriedResult)(1 | 0)(
  2 | 0
)(42 | 0);

export const partialRun = Data_Function_Uncurried.runFn2(chooseSecond)(1 | 0);

export const indirectMake = Data_Function_Uncurried.mkFn2(chooseSecondCurried);

export const lookalikeMade = (() => {
  return Lookalike.mkFn2(first => second => second);
})();

export const lookalikeRun = Lookalike.runFn2(lookalikeMade)(1 | 0)(42 | 0);
