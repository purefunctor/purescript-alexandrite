import * as Data_Eq from "../Data.Eq/index.js";

export function compareTwice(dictionary0) {
  function compareTwice$closure(dictionary0) {
    return left => {
      return right => {
        if (dictionary0.eq(left)(right)) {
          return dictionary0.eq(right)(left);
        } else {
          return false;
        }
      };
    };
  }
  return compareTwice$closure(dictionary0);
}

export function compareIntsTwice(left) {
  return right => {
    if (Data_Eq.eqInt.eq(left)(right)) {
      return Data_Eq.eqInt.eq(right)(left);
    } else {
      return false;
    }
  };
}

export function compareArraysTwice(left) {
  return right => {
    const call = Data_Eq.eqArray(Data_Eq.eqInt);
    if (call.eq(left)(right)) {
      return call.eq(right)(left);
    } else {
      return false;
    }
  };
}

export function compareArraysOnce(left) {
  return right => {
    return (Data_Eq.eqArray(Data_Eq.eqInt)).eq(left)(right);
  };
}

export function compareGenericArraysTwice(dictionary1) {
  function compareGenericArraysTwice$closure(dictionary1) {
    return left => {
      return right => {
        const call = Data_Eq.eqArray(dictionary1);
        if (call.eq(left)(right)) {
          return call.eq(right)(left);
        } else {
          return false;
        }
      };
    };
  }
  return compareGenericArraysTwice$closure(dictionary1);
}

export function compareNestedArraysTwice(left) {
  return right => {
    return nestedLeft => {
      return nestedRight => {
        const call = Data_Eq.eqArray(Data_Eq.eqInt);
        const call$1 = Data_Eq.eqArray(call);
        if (call$1.eq(nestedLeft)(nestedRight)) {
          return call$1.eq(nestedRight)(nestedLeft);
        } else {
          return call.eq(left)(right);
        }
      };
    };
  };
}

export function distinctGivens(dictionary2) {
  return dictionary3 => {
    function distinctGivens$closure(dictionary2, dictionary3) {
      return leftA => {
        return rightA => {
          return leftB => {
            return rightB => {
              function if$join$1(result$1) {
                return result$1;
              }

              const call = Data_Eq.eqArray(dictionary2);
              const call$1 = Data_Eq.eqArray(dictionary3);
              if (call.eq(leftA)(rightA)) {
                return call.eq(rightA)(leftA);
              } else {
                if (call$1.eq(leftB)(rightB)) {
                  return if$join$1(call$1.eq(rightB)(leftB));
                } else {
                  return if$join$1(false);
                }
              }
            };
          };
        };
      };
    }
    return distinctGivens$closure(dictionary2, dictionary3);
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
        if (call.eq(leftInt)(rightInt)) {
          return call.eq(rightInt)(leftInt);
        } else {
          if (call$1.eq(leftBoolean)(rightBoolean)) {
            return if$join$1(call$1.eq(rightBoolean)(leftBoolean));
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
    if (call.eq(left)(right)) {
      if (call.eq(right)(left)) {
        return if$join$1(call.eq(left)(right));
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
    if (call$1.eq(left)(right)) {
      return call$1.eq(right)(left);
    } else {
      return false;
    }
  };
}

export function compareSuperclassArraysTwice(dictionary4) {
  function compareSuperclassArraysTwice$closure(dictionary4) {
    return left => {
      return right => {
        const call = Data_Eq.eqArray(dictionary4.superclass62);
        if (call.eq(left)(right)) {
          return call.eq(right)(left);
        } else {
          return false;
        }
      };
    };
  }
  return compareSuperclassArraysTwice$closure(dictionary4);
}

export function compareSuperclassTwice(dictionary5) {
  function compareSuperclassTwice$closure(dictionary5) {
    return left => {
      return right => {
        if (dictionary5.superclass62.eq(left)(right)) {
          return dictionary5.superclass62.eq(right)(left);
        } else {
          return false;
        }
      };
    };
  }
  return compareSuperclassTwice$closure(dictionary5);
}

export function lambdaScope(left) {
  return right => {
    if ((Data_Eq.eqArray(Data_Eq.eqInt)).eq(left)(right)) {
      function lambdaScope$closure(lambdaLeft) {
        return lambdaRight => {
          const call = Data_Eq.eqArray(Data_Eq.eqInt);
          if (call.eq(lambdaLeft)(lambdaRight)) {
            return call.eq(lambdaRight)(lambdaLeft);
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
    function whereIsolation$closure(helperLeft) {
      return helperRight => {
        return (Data_Eq.eqArray(Data_Eq.eqInt)).eq(helperLeft)(helperRight);
      };
    }
    if (whereIsolation$closure(left)(right)) {
      return (Data_Eq.eqArray(Data_Eq.eqInt)).eq(left)(right);
    } else {
      return false;
    }
  };
}

export function equationScope(argument0) {
  return argument1 => {
    return argument2 => {
      const call = Data_Eq.eqArray(Data_Eq.eqInt);
      if (argument0 === true) {
        return call.eq(argument1)(argument2);
      } else {
        if (argument0 === false) {
          return call.eq(argument2)(argument1);
        } else {
          throw new Error("Pattern match failure");
        }
      }
    };
  };
}

export const firstComparison = Data_Eq.eqInt.eq(1 | 0)(2 | 0);

export const secondComparison = Data_Eq.eqInt.eq(3 | 0)(4 | 0);
