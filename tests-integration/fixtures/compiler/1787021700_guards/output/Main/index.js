export const Empty = "Empty";
export const One = ($value0) => ({
  tag: "One",
  _1: $value0
});
export function booleanGuard(value) {
  if (value) {
    return 1 | 0;
  }
  if (true) {
    return 0 | 0;
  }
  throw new Error("Pattern match failure");
}
export function patternGuard(choice) {
  if (choice.tag === "One") {
    const { _1: value } = choice;
    return value;
  }
  if (true) {
    return 0 | 0;
  }
  throw new Error("Pattern match failure");
}
export function caseBooleanGuard(value) {
  {
    if (false) {
      return 1 | 0;
    }
  }
  return 2 | 0;
}
export function casePatternGuard(choice) {
  {
    if (choice.tag === "One") {
      const { _1: value } = choice;
      return value;
    }
  }
  return 0 | 0;
}
export function nestedCaseGuard(value) {
  if (value === true) {
    if (true) {
      {
        if (false) {
          return 1 | 0;
        }
      }
      return 2 | 0;
    }
  }
  return 3 | 0;
}
