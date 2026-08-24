export function use(namedBodyDict) {
  function use$closure(namedBodyDict) {
    return $body => {
      return (namedBodyDict => namedBodyDict.name)(namedBodyDict);
    };
  }
  return use$closure(namedBodyDict);
}
