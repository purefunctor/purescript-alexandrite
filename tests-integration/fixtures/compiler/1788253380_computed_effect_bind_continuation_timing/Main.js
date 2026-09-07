let constructions = 0;

export const makeContinuation = () => {
  constructions += 1;
  return () => () => 42;
};

export const resetConstructions = () => {
  constructions = 0;
};

export const readConstructions = () => constructions;
