export function choose(first) {
  return second => {
    function case$1(first, second) {
      const matches$2 = first === true;
      if (matches$2) {
        const matches$3 = second === false;
        if (matches$3) {
          return 1 | 0;
        } else {
          return case$2(first);
        }
      } else {
        return case$2(first);
      }
    }

    function case$2(first) {
      const matches$4 = first === false;
      if (matches$4) {
        return 0 | 0;
      } else {
        throw new Error("Pattern match failure");
      }
    }

    const matches = first === true;
    if (matches) {
      const matches$1 = second === true;
      if (matches$1) {
        return 2 | 0;
      } else {
        return case$1(first, second);
      }
    } else {
      return case$1(first, second);
    }
  };
}
