export const Pair = $value0 => $value1 => ["Pair", $value0, $value1];

export function identity(value) {
  return value;
}

export const use = (() => {
  const alias = identity;
  return Pair(alias(42 | 0))(alias("x"));
})();
