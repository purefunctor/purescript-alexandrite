export const Just = ($value0) => ({
  tag: "Just",
  _1: $value0
});
export const Nothing = "Nothing";
export function complete(section15) {
  return (section16) => {
    if (section15.tag === "Just" && section16.tag === "Just") {
      return 1 | 0;
    }
    if (section15.tag === "Just" && section16 === "Nothing") {
      return 2 | 0;
    }
    if (section15 === "Nothing" && section16.tag === "Just") {
      return 3 | 0;
    }
    if (section15 === "Nothing" && section16 === "Nothing") {
      return 4 | 0;
    }
    throw new Error("Pattern match failure");
  };
}
export function incomplete1(partialDict) {
  const $closure = (section55) => {
    return (section56) => {
      let $result;
      $case: {
        if (section55.tag === "Just" && section56 === "Nothing") {
          $result = 2 | 0;
          break $case;
        }
        if (section55 === "Nothing" && section56.tag === "Just") {
          $result = 3 | 0;
          break $case;
        }
        if (section55 === "Nothing" && section56 === "Nothing") {
          $result = 4 | 0;
          break $case;
        }
        throw new Error("Pattern match failure");
      }
      return /* @__PURE__ */ $result(partialDict);
    };
  };
  return $closure;
}
export function incomplete2(partialDict) {
  const $closure = (section86) => {
    return (section87) => {
      let $result;
      $case: {
        if (section86.tag === "Just" && section87.tag === "Just") {
          $result = 1 | 0;
          break $case;
        }
        if (section86 === "Nothing" && section87.tag === "Just") {
          $result = 3 | 0;
          break $case;
        }
        if (section86 === "Nothing" && section87 === "Nothing") {
          $result = 4 | 0;
          break $case;
        }
        throw new Error("Pattern match failure");
      }
      return /* @__PURE__ */ $result(partialDict);
    };
  };
  return $closure;
}
export function incomplete3(partialDict) {
  const $closure = (section118) => {
    return (section119) => {
      let $result;
      $case: {
        if (section118.tag === "Just" && section119.tag === "Just") {
          $result = 1 | 0;
          break $case;
        }
        if (section118.tag === "Just" && section119 === "Nothing") {
          $result = 2 | 0;
          break $case;
        }
        if (section118 === "Nothing" && section119 === "Nothing") {
          $result = 4 | 0;
          break $case;
        }
        throw new Error("Pattern match failure");
      }
      return /* @__PURE__ */ $result(partialDict);
    };
  };
  return $closure;
}
export function incomplete4(partialDict) {
  const $closure = (section150) => {
    return (section151) => {
      let $result;
      $case: {
        if (section150.tag === "Just" && section151.tag === "Just") {
          $result = 1 | 0;
          break $case;
        }
        if (section150.tag === "Just" && section151 === "Nothing") {
          $result = 2 | 0;
          break $case;
        }
        if (section150 === "Nothing" && section151.tag === "Just") {
          $result = 3 | 0;
          break $case;
        }
        throw new Error("Pattern match failure");
      }
      return /* @__PURE__ */ $result(partialDict);
    };
  };
  return $closure;
}
