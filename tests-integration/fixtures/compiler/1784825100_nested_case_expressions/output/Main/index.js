export const Empty = "Empty";
export const One = ($value0) => ({
  tag: "One",
  _1: $value0
});
export const Pair = ($value0) => ($value1) => ({
  tag: "Pair",
  _1: $value0,
  _2: $value1
});
export function identity(value) {
  return value;
}
export function nested(choice) {
  if (choice.tag === "One") {
    const { _1: inner } = choice;
    if (inner.tag === "One") {
      const { _1: value } = inner;
      return value;
    }
    if (inner.tag === "Pair") {
      const { _1: left } = inner;
      return left;
    }
    return 0 | 0;
  }
  return 0 | 0;
}
export function applied(choice) {
  let $result;
  $case: {
    if (choice.tag === "One") {
      const { _1: value } = choice;
      $result = value;
      break $case;
    }
    $result = 0 | 0;
    break $case;
  }
  return identity($result);
}
