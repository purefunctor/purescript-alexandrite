import * as Data_Eq from "../Data.Eq/index.js";

export function compareTwice(eqADict) {
  function compareTwice$closure(eqADict) {
    return left => {
      return right => {
        if (Data_Eq.eq(eqADict)(left)(right)) {
          return Data_Eq.eq(eqADict)(right)(left);
        } else {
          return false;
        }
      };
    };
  }
  return compareTwice$closure(eqADict);
}

export function compareIntsTwice(left) {
  return right => {
    if (Data_Eq.eq(Data_Eq.eqInt)(left)(right)) {
      return Data_Eq.eq(Data_Eq.eqInt)(right)(left);
    } else {
      return false;
    }
  };
}

export function compareArraysTwice(left) {
  return right => {
    const call = Data_Eq.eqArray(Data_Eq.eqInt);
    if (Data_Eq.eq(call)(left)(right)) {
      return Data_Eq.eq(call)(right)(left);
    } else {
      return false;
    }
  };
}

export function compareArraysOnce(left) {
  return right => {
    return Data_Eq.eq(Data_Eq.eqArray(Data_Eq.eqInt))(left)(right);
  };
}

export function compareGenericArraysTwice(eqADict) {
  function compareGenericArraysTwice$closure(eqADict) {
    return left => {
      return right => {
        const call = Data_Eq.eqArray(eqADict);
        if (Data_Eq.eq(call)(left)(right)) {
          return Data_Eq.eq(call)(right)(left);
        } else {
          return false;
        }
      };
    };
  }
  return compareGenericArraysTwice$closure(eqADict);
}

export function compareNestedArraysTwice(left) {
  return right => {
    return nestedLeft => {
      return nestedRight => {
        const call = Data_Eq.eqArray(Data_Eq.eqInt);
        const call$1 = Data_Eq.eqArray(call);
        if (Data_Eq.eq(call$1)(nestedLeft)(nestedRight)) {
          return Data_Eq.eq(call$1)(nestedRight)(nestedLeft);
        } else {
          return Data_Eq.eq(call)(left)(right);
        }
      };
    };
  };
}

export function distinctGivens(eqADict) {
  return eqBDict => {
    function distinctGivens$closure(eqADict, eqBDict) {
      return leftA => {
        return rightA => {
          return leftB => {
            return rightB => {
              function if$join$1(result$1) {
                return result$1;
              }

              const call = Data_Eq.eqArray(eqADict);
              const call$1 = Data_Eq.eqArray(eqBDict);
              if (Data_Eq.eq(call)(leftA)(rightA)) {
                return Data_Eq.eq(call)(rightA)(leftA);
              } else {
                if (Data_Eq.eq(call$1)(leftB)(rightB)) {
                  return if$join$1(Data_Eq.eq(call$1)(rightB)(leftB));
                } else {
                  return if$join$1(false);
                }
              }
            };
          };
        };
      };
    }
    return distinctGivens$closure(eqADict, eqBDict);
  };
}

export function distinctSubgoals(leftInt) {
  return rightInt => {
    return leftBoolean => {
      return rightBoolean => {
        function if$join$1(result$1) {
          return result$1;
        }

        const call = Data_Eq.eqArray(Data_Eq.eqInt);
        const call$1 = Data_Eq.eqArray(Data_Eq.eqBoolean);
        if (Data_Eq.eq(call)(leftInt)(rightInt)) {
          return Data_Eq.eq(call)(rightInt)(leftInt);
        } else {
          if (Data_Eq.eq(call$1)(leftBoolean)(rightBoolean)) {
            return if$join$1(Data_Eq.eq(call$1)(rightBoolean)(leftBoolean));
          } else {
            return if$join$1(false);
          }
        }
      };
    };
  };
}

export function compareArraysThrice(left) {
  return right => {
    function if$join$1(result$1) {
      return result$1;
    }

    const call = Data_Eq.eqArray(Data_Eq.eqInt);
    if (Data_Eq.eq(call)(left)(right)) {
      if (Data_Eq.eq(call)(right)(left)) {
        return if$join$1(Data_Eq.eq(call)(left)(right));
      } else {
        return if$join$1(false);
      }
    } else {
      return false;
    }
  };
}

export function compareNestedArraysWhole(left) {
  return right => {
    const call$1 = Data_Eq.eqArray(Data_Eq.eqArray(Data_Eq.eqInt));
    if (Data_Eq.eq(call$1)(left)(right)) {
      return Data_Eq.eq(call$1)(right)(left);
    } else {
      return false;
    }
  };
}

export function compareSuperclassArraysTwice(orderedADict) {
  function compareSuperclassArraysTwice$closure(orderedADict) {
    return left => {
      return right => {
        const call$1 = Data_Eq.eqArray(orderedADict.Eq0());
        if (Data_Eq.eq(call$1)(left)(right)) {
          return Data_Eq.eq(call$1)(right)(left);
        } else {
          return false;
        }
      };
    };
  }
  return compareSuperclassArraysTwice$closure(orderedADict);
}

export function compareSuperclassTwice(orderedADict) {
  function compareSuperclassTwice$closure(orderedADict) {
    return left => {
      return right => {
        const call = orderedADict.Eq0();
        if (Data_Eq.eq(call)(left)(right)) {
          return Data_Eq.eq(call)(right)(left);
        } else {
          return false;
        }
      };
    };
  }
  return compareSuperclassTwice$closure(orderedADict);
}

export function lambdaScope(left) {
  return right => {
    if (Data_Eq.eq(Data_Eq.eqArray(Data_Eq.eqInt))(left)(right)) {
      function lambdaScope$closure(lambdaLeft) {
        return lambdaRight => {
          const call = Data_Eq.eqArray(Data_Eq.eqInt);
          if (Data_Eq.eq(call)(lambdaLeft)(lambdaRight)) {
            return Data_Eq.eq(call)(lambdaRight)(lambdaLeft);
          } else {
            return false;
          }
        };
      }
      return lambdaScope$closure(left)(right);
    } else {
      return false;
    }
  };
}

export function whereIsolation(left) {
  return right => {
    if ((helperLeft => helperRight => Data_Eq.eq(Data_Eq.eqArray(Data_Eq.eqInt))(helperLeft)(
      helperRight
    ))(left)(right)) {
      return Data_Eq.eq(Data_Eq.eqArray(Data_Eq.eqInt))(left)(right);
    } else {
      return false;
    }
  };
}

export function equationScope($boolean) {
  return $array => {
    return $array$1 => {
      const call = Data_Eq.eqArray(Data_Eq.eqInt);
      if ($boolean === true) {
        return Data_Eq.eq(call)($array)($array$1);
      } else {
        if ($boolean === false) {
          return Data_Eq.eq(call)($array$1)($array);
        } else {
          throw new Error("Pattern match failure");
        }
      }
    };
  };
}

export const firstComparison = Data_Eq.eq(Data_Eq.eqInt)(1 | 0)(2 | 0);

export const secondComparison = Data_Eq.eq(Data_Eq.eqInt)(3 | 0)(4 | 0);
