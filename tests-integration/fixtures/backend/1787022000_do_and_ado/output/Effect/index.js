import * as $foreign from "./foreign.js";

export const mapEffect = $foreign["mapEffect"];
export const applyEffect = $foreign["applyEffect"];
export const pureEffect = $foreign["pureEffect"];
export const bindEffect = $foreign["bindEffect"];

export const functorEffect = { map: mapEffect };

export const applyEffect1 = (() => {
  return { Functor0: () => functorEffect, apply: applyEffect };
})();

export const applicativeEffect = (() => {
  return { Apply0: () => applyEffect1, pure: pureEffect };
})();

export const bindEffect1 = (() => {
  return { Apply0: () => applyEffect1, bind: bindEffect };
})();

export const monadEffect = (() => {
  return { Applicative0: () => applicativeEffect, Bind1: () => bindEffect1 };
})();
