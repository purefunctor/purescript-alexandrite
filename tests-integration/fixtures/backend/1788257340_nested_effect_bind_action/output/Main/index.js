import * as Data_Unit from "../Data.Unit/index.js";
export function nestedBind($unit) {
  const $value = Data_Unit.unit;
  return () => {
    let value;
    value = 42 | 0;
    return value;
  };
}
