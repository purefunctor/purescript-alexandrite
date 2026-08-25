export function empty(dictionary) {
  return dictionary.empty;
}

export function chooseEmpty(emptyCollectionDict) {
  function chooseEmpty$closure(emptyCollectionDict) {
    return section33 => {
      if (section33 === true) {
        return empty(emptyCollectionDict);
      } else {
        if (section33 === false) {
          return empty(emptyCollectionDict);
        } else {
          throw new Error("Pattern match failure");
        }
      }
    };
  }
  return chooseEmpty$closure(emptyCollectionDict);
}
