import * as $foreign from "./foreign.js";
export const Unit = "Unit";
export const Proxy = "Proxy";
export const consume = $foreign["consume"];
export const test = consume(($unit) => (markerValueDict) => (extra) => "Proxy");
