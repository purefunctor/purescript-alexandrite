export const Pair = $value0 => $value1 => ["Pair", $value0, $value1];

export function identity(value) {
  return value;
}

export const use = (() => {
  const identity$1 = identity;
  return Pair(identity$1(42 | 0))(identity$1("x"));
})();
