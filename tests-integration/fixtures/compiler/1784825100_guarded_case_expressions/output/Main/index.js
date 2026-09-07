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
export function guarded(condition) {
  return (choice) => {
    if (choice.tag === "One") {
      const { _1: nested } = choice;
      if (condition) {
        if (nested.tag === "One") {
          const { _1: value } = nested;
          return value;
        }
      }
    }
    if (choice.tag === "Pair") {
      const { _1: left, _2: right } = choice;
      if (left.tag === "One") {
        const { _1: value$1 } = left;
        return value$1;
      }
      if (right.tag === "One") {
        const { _1: value$2 } = right;
        return value$2;
      }
    }
    return 0 | 0;
  };
}
export function withWhere(choice) {
  if (choice.tag === "One") {
    const { _1: value } = choice;
    return value;
  }
  return 0 | 0;
}
