import * as $runtime from "../runtime.js";

export const Box = $value0 => ["Box", $value0];

export function liftApplicative(applicativeFunctorDict) {
  return $function => value => (applicativeFunctorDict.Apply0()).apply(
    applicativeFunctorDict.pure($function)
  )(value);
}

export function applyMonad(monadMonadDict) {
  function applyMonad$closure(monadMonadDict) {
    return functions => {
      return values => {
        function applyMonad$closure$closure(monadMonadDict, values) {
          return $function => {
            return (monadMonadDict.Bind1()).bind(values)(
              value => (monadMonadDict.Applicative0()).pure($function(value))
            );
          };
        }
        return (monadMonadDict.Bind1()).bind(functions)(
          applyMonad$closure$closure(monadMonadDict, values)
        );
      };
    };
  }
  return applyMonad$closure(monadMonadDict);
}

const $lazy_functorBox = $runtime.binding("functorBox", () => {
  return { map: liftApplicative($lazy_applicativeBox()) };
});

const $lazy_applyBox = $runtime.binding("applyBox", () => {
  return { Functor0: () => $lazy_functorBox(), apply: applyMonad($lazy_monadBox()) };
});

const $lazy_applicativeBox = $runtime.binding("applicativeBox", () => {
  return { Apply0: () => $lazy_applyBox(), pure: Box };
});

const $lazy_bindBox = $runtime.binding("bindBox", () => {
  function bindBox$initialize$closure$1($box) {
    return continuation => {
      if (Array.isArray($box) && $box[0] === "Box") {
        return continuation($box[1]);
      } else {
        throw new Error("Pattern match failure");
      }
    };
  }
  return { Apply0: () => $lazy_applyBox(), bind: bindBox$initialize$closure$1 };
});

const $lazy_monadBox = $runtime.binding("monadBox", () => {
  return { Applicative0: () => $lazy_applicativeBox(), Bind1: () => $lazy_bindBox() };
});

export const functorBox = $lazy_functorBox();

export const applyBox = $lazy_applyBox();

export const applicativeBox = $lazy_applicativeBox();

export const bindBox = $lazy_bindBox();

export const monadBox = $lazy_monadBox();

export const result = (() => {
  const call$2 = ($lazy_functorBox()).map(value => value)(Box(42 | 0));
  if (Array.isArray(call$2) && call$2[0] === "Box") {
    return call$2[1];
  } else {
    throw new Error("Pattern match failure");
  }
})();
