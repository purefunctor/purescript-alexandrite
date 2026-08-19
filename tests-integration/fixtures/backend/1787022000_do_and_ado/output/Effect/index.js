import * as $foreign from "./foreign.js";

export const mapEffect = $foreign["mapEffect"];
export const applyEffect = $foreign["applyEffect"];
export const pureEffect = $foreign["pureEffect"];
export const bindEffect = $foreign["bindEffect"];

export const functorEffect = { map: mapEffect };

export const applyEffect1 = { superclass14: functorEffect, apply: applyEffect };

export const applicativeEffect = { superclass14: applyEffect1, pure: pureEffect };

export const bindEffect1 = { superclass19: applyEffect1, bind: bindEffect };

export const monadEffect = { superclass19: applicativeEffect, superclass22: bindEffect1 };
