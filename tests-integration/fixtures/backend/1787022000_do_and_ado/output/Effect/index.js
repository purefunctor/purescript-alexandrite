import * as $foreign from "./foreign.js";

export const mapEffect = $foreign["mapEffect"];
export const applyEffect = $foreign["applyEffect"];
export const pureEffect = $foreign["pureEffect"];
export const bindEffect = $foreign["bindEffect"];

export const functorEffect = { map: mapEffect };

export const applyEffect1 = { Functor0: functorEffect, apply: applyEffect };

export const applicativeEffect = { Apply0: applyEffect1, pure: pureEffect };

export const bindEffect1 = { Apply0: applyEffect1, bind: bindEffect };

export const monadEffect = { Applicative0: applicativeEffect, Bind1: bindEffect1 };
