import * as $foreign from "./foreign.js";
export const mapEffect = $foreign["mapEffect"];
export const applyEffect = $foreign["applyEffect"];
export const pureEffect = $foreign["pureEffect"];
export const bindEffect = $foreign["bindEffect"];
export const functorEffect = { map: mapEffect };
export const applyEffectInstance = {
  Functor0: () => functorEffect,
  apply: applyEffect
};
export const applicativeEffect = {
  Apply0: () => applyEffectInstance,
  pure: pureEffect
};
export const bindEffectInstance = {
  Apply0: () => applyEffectInstance,
  bind: bindEffect
};
export const monadEffect = {
  Applicative0: () => applicativeEffect,
  Bind1: () => bindEffectInstance
};
