import * as $foreign from "./foreign.js";
export const pure = $foreign["pure"];
export const bind = $foreign["bind"];
export const discard = $foreign["discard"];
export const bound = bind(pure(1 | 0))((value) => pure(value));
export const discarded = discard(pure(1 | 0))(($int) => pure("done"));
export const withLet = bind(pure(1 | 0))((value) => pure(value));
export const nested = bind(bind(pure(1 | 0))((inner) => pure(inner)))((value) => pure(value));
