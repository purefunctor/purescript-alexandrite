export function eq(dictionary) {
  return dictionary.eq;
}

export function eq1(dictionary) {
  return dictionary.eq1;
}

export function eqRec(rowToListRowListDict) {
  return eqRecordListRowDict => {
    return { eq: $record => $record$1 => true };
  };
}

export function eqRecord(dictionary) {
  return dictionary.eqRecord;
}

export function eqRecordConsType(eqRecordRowlistTailRowDict) {
  return consKeyFocusRowTailRowDict => {
    return isSymbolKeyDict => {
      return eqFocusDict => {
        return { eqRecord: $cons => $record => $record$1 => true };
      };
    };
  };
}

export const eqInt = (() => {
  return { eq: $int => $int$1 => true };
})();

export const eqBoolean = (() => {
  return { eq: $boolean => $boolean$1 => true };
})();

export const eqRecordNilType = (() => {
  return { eqRecord: $nil => $record => $record$1 => true };
})();
