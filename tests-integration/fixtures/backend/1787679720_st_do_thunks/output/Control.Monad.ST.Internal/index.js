import * as $foreign from "./foreign.js";

export const map_ = $foreign["map_"];
export const pure_ = $foreign["pure_"];
export const bind_ = $foreign["bind_"];
export const run = $foreign["run"];

export const functorST = { map: map_ };

export const applyST = {
  Functor0: () => functorST,
  apply: functionAction => argumentAction => bind_(functionAction)(
    $function => bind_(argumentAction)(argument => pure_($function(argument)))
  )
};

export const applicativeST = { Apply0: () => applyST, pure: pure_ };

export const bindST = { Apply0: () => applyST, bind: bind_ };

export const monadST = { Applicative0: () => applicativeST, Bind1: () => bindST };
