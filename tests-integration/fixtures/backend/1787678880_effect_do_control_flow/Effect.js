export const mapE = transform => action => () => transform(action());
export const applyE = functionAction => valueAction => () => functionAction()(valueAction());
export const pureE = value => () => value;
export const bindE = action => continuation => () => continuation(action())();
