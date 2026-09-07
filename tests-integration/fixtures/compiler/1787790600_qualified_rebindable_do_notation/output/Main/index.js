import * as Control from "../Control/index.js";
export const qualifiedDo = Control.bind(Control.action)((value) => Control.discard(Control.action)(($argument0) => Control.pure(value)));
export const qualifiedAdo = Control.apply(Control.map((left) => (right) => left)(Control.action))(Control.action);
export const qualifiedPure = Control.pure(1 | 0);
