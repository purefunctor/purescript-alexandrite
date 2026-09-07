export function add(left) {
  return (right) => {
    return left;
  };
}
export function use(integer) {
  return (string) => {
    return string;
  };
}
export function checked(section40) {
  return add(section40)(1 | 0);
}
export function inferred(section48) {
  return add(section48)(1 | 0);
}
export function ordered(section64) {
  return (section66) => {
    return use(section64)(section66);
  };
}
export function nested(section71) {
  return section71((section75) => section75(1 | 0));
}
export function access(section82) {
  return section82.value;
}
export function update(section88) {
  return (section91) => {
    return (section93) => {
      return {
        ...section88,
        first: section91,
        second: section93
      };
    };
  };
}
export function conditional(section99) {
  return (section101) => {
    return (section103) => {
      if (section99) {
        return section101;
      } else {
        return section103;
      }
    };
  };
}
export function caseScrutinee(section109) {
  if (section109 === true) {
    return 1 | 0;
  }
  if (section109 === false) {
    return 0 | 0;
  }
  throw new Error("Pattern match failure");
}
export function caseScrutinees(section128) {
  return (section129) => {
    if (section128 === true) {
      const value = section129;
      return value;
    }
    if (section128 === false) {
      return 0 | 0;
    }
    throw new Error("Pattern match failure");
  };
}
export function caseAlternatives(section150) {
  if (section150 === true) {
    return (section158) => section158.value;
  }
  if (section150 === false) {
    return (section166) => section166.value;
  }
  throw new Error("Pattern match failure");
}
