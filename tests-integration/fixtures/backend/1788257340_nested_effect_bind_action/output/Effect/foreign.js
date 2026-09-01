export const mapEffect = transform => action => () => transform(action());
export const applyEffect = functionAction => valueAction => () => functionAction()(valueAction());
export const pureEffect = value => () => value;
export const bindEffect = action => continuation => () => continuation(action())();
