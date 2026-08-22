import * as $runtime from "../runtime.js";

export const Box = $value0 => ["Box", $value0];

export function liftApplicative(applicativeFunctorDict) {
  function liftApplicative$closure(applicativeFunctorDict) {
    return $function => {
      return value => {
        return (applicativeFunctorDict.Apply0({})).apply(applicativeFunctorDict.pure($function))(
          value
        );
      };
    };
  }
  return liftApplicative$closure(applicativeFunctorDict);
}

export function applyMonad(monadMonadDict) {
  function applyMonad$closure(monadMonadDict) {
    return functions => {
      return values => {
        function applyMonad$closure$closure(monadMonadDict, values) {
          return $function => {
            function applyMonad$closure$closure$closure(monadMonadDict, $function) {
              return value => {
                return (monadMonadDict.Applicative0({})).pure($function(value));
              };
            }
            return (monadMonadDict.Bind1({})).bind(values)(
              applyMonad$closure$closure$closure(monadMonadDict, $function)
            );
          };
        }
        return (monadMonadDict.Bind1({})).bind(functions)(
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
  function applyBox$initialize$closure(unit) {
    return $lazy_functorBox();
  }
  return { Functor0: applyBox$initialize$closure, apply: applyMonad($lazy_monadBox()) };
});

const $lazy_applicativeBox = $runtime.binding("applicativeBox", () => {
  function applicativeBox$initialize$closure(unit) {
    return $lazy_applyBox();
  }
  return { Apply0: applicativeBox$initialize$closure, pure: Box };
});

const $lazy_bindBox = $runtime.binding("bindBox", () => {
  function bindBox$initialize$closure(unit) {
    return $lazy_applyBox();
  }
  function bindBox$initialize$closure$1($box) {
    return continuation => {
      if (Array.isArray($box) && $box[0] === "Box") {
        return continuation($box[1]);
      } else {
        throw new Error("Pattern match failure");
      }
    };
  }
  return { Apply0: bindBox$initialize$closure, bind: bindBox$initialize$closure$1 };
});

const $lazy_monadBox = $runtime.binding("monadBox", () => {
  function monadBox$initialize$closure(unit) {
    return $lazy_applicativeBox();
  }
  function monadBox$initialize$closure$1(unit) {
    return $lazy_bindBox();
  }
  return { Applicative0: monadBox$initialize$closure, Bind1: monadBox$initialize$closure$1 };
});

export const functorBox = $lazy_functorBox();

export const applyBox = $lazy_applyBox();

export const applicativeBox = $lazy_applicativeBox();

export const bindBox = $lazy_bindBox();

export const monadBox = $lazy_monadBox();

export const result = (() => {
  function result$initialize$closure(value) {
    return value;
  }
  const call$2 = ($lazy_functorBox()).map(result$initialize$closure)(Box(42 | 0));
  if (Array.isArray(call$2) && call$2[0] === "Box") {
    return call$2[1];
  } else {
    throw new Error("Pattern match failure");
  }
})();
