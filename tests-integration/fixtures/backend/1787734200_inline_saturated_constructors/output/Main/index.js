import * as Library from "../Library/index.js";
export const Local = ($value0) => ($value1) => [
  "Local",
  $value0,
  $value1
];
export const Empty = "Empty";
export const local = Local(1 | 0)("local");
export const partial = Local(2 | 0);
export const empty = Empty;
export const external = Library.External("external");
