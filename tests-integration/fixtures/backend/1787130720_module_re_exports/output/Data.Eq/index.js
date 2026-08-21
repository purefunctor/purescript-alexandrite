export function eqRec(rowToListRowListDict) {
  return eqRecordListRowDict => {
    function eqRec$closure($record) {
      return $record$1 => {
        return true;
      };
    }
    return { eq: eqRec$closure };
  };
}

export function eqRecordConsType(eqRecordRowlistTailRowDict) {
  return consKeyFocusRowTailRowDict => {
    return isSymbolKeyDict => {
      return eqFocusDict => {
        function eqRecordConsType$closure($cons) {
          return $record => {
            return $record$1 => {
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
  function eqInt$initialize$closure($int) {
    return $int$1 => {
      return true;
    };
  }
  return { eq: eqInt$initialize$closure };
})();

export const eqBoolean = (() => {
  function eqBoolean$initialize$closure($boolean) {
    return $boolean$1 => {
      return true;
    };
  }
  return { eq: eqBoolean$initialize$closure };
})();

export const eqRecordNilType = (() => {
  function eqRecordNilType$initialize$closure($nil) {
    return $record => {
      return $record$1 => {
        return true;
      };
    };
  }
  return { eqRecord: eqRecordNilType$initialize$closure };
})();
