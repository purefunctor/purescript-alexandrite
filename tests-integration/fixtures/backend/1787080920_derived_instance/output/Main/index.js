import * as Data_Eq from "../Data.Eq/index.js";
import * as Data_Show from "../Data.Show/index.js";

export const Box = ["Box"];

export function showIdentity(showADict) {
  return showADict;
}

export const eqBox = (() => {
  const $closure = left => {
    return right => {
      if (Array.isArray(left) && left[0] === "Box" && Array.isArray(right) && right[0] === "Box") {
        return true;
      }
      throw new Error("Pattern match failure");
    };
  };
  const $field = $closure;
  const $record = { eq: $field };
  return $record;
})();

export const equal = Data_Eq.eq(eqBox)(Box)(Box);

export const rendered = Data_Show.show(showIdentity(Data_Show.showInt))(42 | 0);
