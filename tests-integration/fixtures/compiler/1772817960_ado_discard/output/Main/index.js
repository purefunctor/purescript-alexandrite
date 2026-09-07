import * as $foreign from "./foreign.js";
export const Unit = "Unit";
export const pure = $foreign["pure"];
export const map = $foreign["map"];
export const apply = $foreign["apply"];
export const unit = "Unit";
export const test = apply(map(($int) => ($unit) => unit)(pure(1 | 0)))(pure(unit));
const test_ = apply(map(($int) => ($unit) => unit)(pure(1 | 0)))(pure(unit));
export { test_ as "test'" };
