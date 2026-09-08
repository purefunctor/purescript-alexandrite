export const Tuple = ($value0) => ($value1) => ({
  tag: "Tuple",
  _1: $value0,
  _2: $value1
});
export const Just = ($value0) => ({
  tag: "Just",
  _1: $value0
});
export const Nothing = "Nothing";
export function complete(section21) {
  if (section21.tag === "Tuple" && section21._1.tag === "Just" && section21._2.tag === "Just") {
    return 1 | 0;
  }
  if (section21.tag === "Tuple" && section21._1.tag === "Just" && section21._2 === "Nothing") {
    return 2 | 0;
  }
  if (section21.tag === "Tuple" && section21._1 === "Nothing" && section21._2.tag === "Just") {
    return 3 | 0;
  }
  if (section21.tag === "Tuple" && section21._1 === "Nothing" && section21._2 === "Nothing") {
    return 4 | 0;
  }
  throw new Error("Pattern match failure");
}
export function incomplete1(partialDict) {
  const $closure = (section68) => {
    let $result;
    $case: {
      if (section68.tag === "Tuple" && section68._1.tag === "Just" && section68._2 === "Nothing") {
        $result = 2 | 0;
        break $case;
      }
      if (section68.tag === "Tuple" && section68._1 === "Nothing" && section68._2.tag === "Just") {
        $result = 3 | 0;
        break $case;
      }
      if (section68.tag === "Tuple" && section68._1 === "Nothing" && section68._2 === "Nothing") {
        $result = 4 | 0;
        break $case;
      }
      throw new Error("Pattern match failure");
    }
    return /* @__PURE__ */ $result(partialDict);
  };
  return $closure;
}
export function incomplete2(partialDict) {
  const $closure = (section103) => {
    let $result;
    $case: {
      if (section103.tag === "Tuple" && section103._1.tag === "Just" && section103._2.tag === "Just") {
        $result = 1 | 0;
        break $case;
      }
      if (section103.tag === "Tuple" && section103._1 === "Nothing" && section103._2.tag === "Just") {
        $result = 3 | 0;
        break $case;
      }
      if (section103.tag === "Tuple" && section103._1 === "Nothing" && section103._2 === "Nothing") {
        $result = 4 | 0;
        break $case;
      }
      throw new Error("Pattern match failure");
    }
    return /* @__PURE__ */ $result(partialDict);
  };
  return $closure;
}
export function incomplete3(partialDict) {
  const $closure = (section140) => {
    let $result;
    $case: {
      if (section140.tag === "Tuple" && section140._1.tag === "Just" && section140._2.tag === "Just") {
        $result = 1 | 0;
        break $case;
      }
      if (section140.tag === "Tuple" && section140._1.tag === "Just" && section140._2 === "Nothing") {
        $result = 2 | 0;
        break $case;
      }
      if (section140.tag === "Tuple" && section140._1 === "Nothing" && section140._2 === "Nothing") {
        $result = 4 | 0;
        break $case;
      }
      throw new Error("Pattern match failure");
    }
    return /* @__PURE__ */ $result(partialDict);
  };
  return $closure;
}
export function incomplete4(partialDict) {
  const $closure = (section177) => {
    let $result;
    $case: {
      if (section177.tag === "Tuple" && section177._1.tag === "Just" && section177._2.tag === "Just") {
        $result = 1 | 0;
        break $case;
      }
      if (section177.tag === "Tuple" && section177._1.tag === "Just" && section177._2 === "Nothing") {
        $result = 2 | 0;
        break $case;
      }
      if (section177.tag === "Tuple" && section177._1 === "Nothing" && section177._2.tag === "Just") {
        $result = 3 | 0;
        break $case;
      }
      throw new Error("Pattern match failure");
    }
    return /* @__PURE__ */ $result(partialDict);
  };
  return $closure;
}
