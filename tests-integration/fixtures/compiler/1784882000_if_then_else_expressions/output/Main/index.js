export function predicate(dictionary) {
  return dictionary.predicate;
}
export function defaultValue(dictionary) {
  return dictionary.defaultValue;
}
export function checked(condition) {
  if (condition) {
    return 1 | 0;
  } else {
    return 2 | 0;
  }
}
export function inferred(condition) {
  if (condition) {
    return "yes";
  } else {
    return "no";
  }
}
export function constrained(predicateValueDict) {
  const $closure = (value) => {
    if (/* @__PURE__ */ predicate(predicateValueDict)(value)) {
      return 1 | 0;
    } else {
      return 2 | 0;
    }
  };
  return $closure;
}
export function constrainedBranches(defaultValueDict) {
  const $closure = (condition) => {
    if (condition) {
      return /* @__PURE__ */ defaultValue(defaultValueDict);
    } else {
      return /* @__PURE__ */ defaultValue(defaultValueDict);
    }
  };
  return $closure;
}
export function higherRank(condition) {
  if (condition) {
    return identity;
  } else {
    return identity;
  }
}
export function nested(first) {
  return (second) => {
    if (first) {
      if (second) {
        return 1 | 0;
      } else {
        return 2 | 0;
      }
    } else {
      return 3 | 0;
    }
  };
}
export function asArgument(condition) {
  let $result;
  if (condition) {
    $result = 1 | 0;
  } else {
    $result = 2 | 0;
  }
  return identity($result);
}
export function identity(value) {
  return value;
}
