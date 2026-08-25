export function use(namedBodyDict) {
  return $body => (namedBodyDict => namedBodyDict.name)(namedBodyDict);
}
