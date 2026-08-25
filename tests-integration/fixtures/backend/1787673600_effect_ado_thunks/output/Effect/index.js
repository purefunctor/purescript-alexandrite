import * as $foreign from "./foreign.js";

export const mapE = $foreign["mapE"];
export const applyE = $foreign["applyE"];
export const pureE = $foreign["pureE"];
export const bindE = $foreign["bindE"];

export const functorEffect = { map: mapE };

export const applyEffect = { Functor0: () => functorEffect, apply: applyE };

export const applicativeEffect = { Apply0: () => applyEffect, pure: pureE };

export const bindEffect = { Apply0: () => applyEffect, bind: bindE };

export const monadEffect = { Applicative0: () => applicativeEffect, Bind1: () => bindEffect };
