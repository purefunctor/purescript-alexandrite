let constructions = 0;

export const observe = value => {
  constructions += 1;
  return value;
};

export const constructionCount = () => constructions;
