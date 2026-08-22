export function chooseEmpty(emptyCollectionDict) {
  function chooseEmpty$closure(emptyCollectionDict) {
    return section33 => {
      if (section33 === true) {
        return emptyCollectionDict.empty;
      } else {
        if (section33 === false) {
          return emptyCollectionDict.empty;
        } else {
          throw new Error("Pattern match failure");
        }
      }
    };
  }
  return chooseEmpty$closure(emptyCollectionDict);
}
