import * as $foreign from "./foreign.js";
export const Tagged = ($value0) => ({
  tag: "Tagged",
  _1: $value0
});
export function readLabel(record) {
  return record["hyphen-label"];
}
export function readEmptyLabel(record) {
  return record[""];
}
const $await = $foreign["await"];
const $arguments = $await;
const $default = { "hyphen-label": $arguments };
export const emptyLabel = { "": $arguments };
export const tagged = {
  tag: "Tagged",
  _1: $default
};
export { $await as "await" };
export { $arguments as "arguments" };
export { $default as "default" };
