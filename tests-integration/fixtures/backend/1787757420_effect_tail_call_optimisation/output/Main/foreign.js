let constructions = 0;
let runs = 0;

export const equalInt = left => right => left === right;
export const decrementInt = value => value - 1;
export const constructTick = () => {
  constructions += 1;
  return () => {
    runs += 1;
  };
};
export const resetCounts = () => {
  constructions = 0;
  runs = 0;
};
export const readCounts = () => ({ constructions, runs });
