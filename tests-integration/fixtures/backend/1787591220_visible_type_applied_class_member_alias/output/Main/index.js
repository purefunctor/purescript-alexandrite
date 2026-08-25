export function name(dictionary) {
  return dictionary.name;
}

export function use(namedBodyDict) {
  const $closure = $body => {
    const alias = name;
    return alias(namedBodyDict);
  };
  return $closure;
}
