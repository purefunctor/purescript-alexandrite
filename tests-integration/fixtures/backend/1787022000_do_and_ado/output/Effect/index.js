import * as $foreign from "./foreign.js";

export const mapEffect = $foreign["mapEffect"];
export const applyEffect = $foreign["applyEffect"];
export const pureEffect = $foreign["pureEffect"];
export const bindEffect = $foreign["bindEffect"];

export const functorEffect = { map: mapEffect };

export const applyEffect1 = (() => {
  function applyEffect1$initialize$closure(unit) {
    return functorEffect;
  }
  return { Functor0: applyEffect1$initialize$closure, apply: applyEffect };
})();

export const applicativeEffect = (() => {
  function applicativeEffect$initialize$closure(unit) {
    return applyEffect1;
  }
  return { Apply0: applicativeEffect$initialize$closure, pure: pureEffect };
})();

export const bindEffect1 = (() => {
  function bindEffect1$initialize$closure(unit) {
    return applyEffect1;
  }
  return { Apply0: bindEffect1$initialize$closure, bind: bindEffect };
})();

export const monadEffect = (() => {
  function monadEffect$initialize$closure(unit) {
    return applicativeEffect;
  }
  function monadEffect$initialize$closure$1(unit) {
    return bindEffect1;
  }
  return { Applicative0: monadEffect$initialize$closure, Bind1: monadEffect$initialize$closure$1 };
})();
