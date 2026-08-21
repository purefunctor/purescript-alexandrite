export function choose(first) {
  return second => {
    function case$1(first, second) {
      if (first === true) {
        if (second === false) {
          return 1 | 0;
        } else {
          return case$2(first);
        }
      } else {
        return case$2(first);
      }
    }

    function case$2(first) {
      if (first === false) {
        return 0 | 0;
      } else {
        throw new Error("Pattern match failure");
      }
    }

    if (first === true) {
      if (second === true) {
        return 2 | 0;
      } else {
        return case$1(first, second);
      }
    } else {
      return case$1(first, second);
    }
  };
}
