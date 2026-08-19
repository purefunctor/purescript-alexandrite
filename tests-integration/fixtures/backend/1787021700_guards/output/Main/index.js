export const Empty = ["Empty"];
export const One = $value0 => ["One", $value0];

export function booleanGuard(value) {
  if (value) {
    const literal = 1 | 0;
    return literal;
  } else {
    const literal$1 = true;
    if (literal$1) {
      const literal$2 = 0 | 0;
      return literal$2;
    } else {
      throw new Error("Pattern match failure");
    }
  }
}

export function patternGuard(choice) {
  const matches = Array.isArray(choice) && choice[0] === "One";
  if (matches) {
    const value = choice[1];
    return value;
  } else {
    const literal = true;
    if (literal) {
      const literal$1 = 0 | 0;
      return literal$1;
    } else {
      throw new Error("Pattern match failure");
    }
  }
}

export function caseBooleanGuard(value) {
  const literal = false;
  if (literal) {
    const literal$1 = 1 | 0;
    const result$1 = literal$1;
    return result$1;
  } else {
    const literal$2 = 2 | 0;
    return literal$2;
  }
}

export function casePatternGuard(choice) {
  const matches = Array.isArray(choice) && choice[0] === "One";
  if (matches) {
    const value = choice[1];
    const result$1 = value;
    return result$1;
  } else {
    const literal = 0 | 0;
    return literal;
  }
}

export function nestedCaseGuard(value) {
  function case$1() {
    const literal$4 = 3 | 0;
    return literal$4;
  }

  function case$join$1(result$2) {
    const result$1 = result$2;
    return result$1;
  }

  const matches = value === true;
  if (matches) {
    const literal = true;
    if (literal) {
      const literal$1 = false;
      if (literal$1) {
        const literal$2 = 1 | 0;
        const result$3 = literal$2;
        return case$join$1(result$3);
      } else {
        const literal$3 = 2 | 0;
        return case$join$1(literal$3);
      }
    } else {
      return case$1();
    }
  } else {
    return case$1();
  }
}
