export const map_ = transform => action => () => transform(action());
export const pure_ = value => () => value;
export const bind_ = action => continuation => () => continuation(action())();
export const run = action => action();
