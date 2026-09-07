import * as $foreign from "./foreign.js";
export const Token = "Token";
export const Effect = ($value0) => ({
  tag: "Effect",
  _1: $value0
});
export function identity(value) {
  return value;
}
export function modify(token) {
  return token;
}
export function applyValue(value) {
  return ($function) => {
    return $function(value);
  };
}
export function escapedSection(section176) {
  return applyValue(leak(modify))(section176);
}
export function escapedRecordUpdate(record) {
  return {
    ...record,
    token: leak(modify)
  };
}
export function escapedPatternGuard(value) {
  const $scrutinee = leak(modify);
  const token = $scrutinee;
  return value;
  throw new Error("Pattern match failure");
}
export function escapedCase(value) {
  const token = value;
  return { token: leak(modify) };
}
export const leak = $foreign["leak"];
export const pure = $foreign["pure"];
export const bind = $foreign["bind"];
export const discard = $foreign["discard"];
export const map = $foreign["map"];
export const apply = $foreign["apply"];
export const escapedOperator = applyValue(leak(modify))(identity);
export const escapedDo = bind(pure(leak(modify)))((token) => pure(token));
export const escapedAdo = map((token) => token)(pure(leak(modify)));
export const escapedRecord = { token: leak(modify) };
