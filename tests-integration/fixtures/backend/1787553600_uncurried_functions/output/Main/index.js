import * as Data_Function_Uncurried from "../Data.Function.Uncurried/index.js";
import * as Lookalike from "../Lookalike/index.js";
import * as $foreign from "./foreign.js";

export function made(first, second) {
  return second;
}

export function madeNested(first, second, third) {
  return third;
}

export function madeCaptured(captured) {
  return (first, second) => captured;
}

export function madeWithCurriedResult(first, second) {
  return third => third;
}

export function chooseSecondCurried(first) {
  return second => {
    return second;
  };
}

export const chooseSecond = $foreign["chooseSecond"];

export const directRun = chooseSecond(1 | 0, 42 | 0);

export const directMadeRun = (() => {
  return ((first, second) => second)(1 | 0, 42 | 0);
})();

export const directNestedRun = madeNested(1 | 0, 2 | 0, 42 | 0);

export const directCapturedRun = madeCaptured(42 | 0)(1 | 0, 2 | 0);

export const directCurriedResultRun = madeWithCurriedResult(1 | 0, 2 | 0)(42 | 0);

export const partialRun = Data_Function_Uncurried.runFn2(chooseSecond)(1 | 0);

export const indirectMake = Data_Function_Uncurried.mkFn2(chooseSecondCurried);

export const lookalikeMade = (() => {
  return Lookalike.mkFn2(first => second => second);
})();

export const lookalikeRun = Lookalike.runFn2(lookalikeMade)(1 | 0)(42 | 0);
