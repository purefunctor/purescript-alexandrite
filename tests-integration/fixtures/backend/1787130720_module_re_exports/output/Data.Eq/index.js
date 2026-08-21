export function eqRec(dictionary0) {
  return dictionary1 => {
    function eqRec$closure(argument0) {
      return argument1 => {
        return true;
      };
    }
    return { eq: eqRec$closure };
  };
}

export function eqRecordConsType(dictionary2) {
  return dictionary3 => {
    return dictionary4 => {
      return dictionary5 => {
        function eqRecordConsType$closure(argument0) {
          return argument1 => {
            return argument2 => {
              return true;
            };
          };
        }
        return { eqRecord: eqRecordConsType$closure };
      };
    };
  };
}

export const eqInt = (() => {
  function eqInt$initialize$closure(argument0) {
    return argument1 => {
      return true;
    };
  }
  return { eq: eqInt$initialize$closure };
})();

export const eqBoolean = (() => {
  function eqBoolean$initialize$closure(argument0) {
    return argument1 => {
      return true;
    };
  }
  return { eq: eqBoolean$initialize$closure };
})();

export const eqRecordNilType = (() => {
  function eqRecordNilType$initialize$closure(argument0) {
    return argument1 => {
      return argument2 => {
        return true;
      };
    };
  }
  return { eqRecord: eqRecordNilType$initialize$closure };
})();
