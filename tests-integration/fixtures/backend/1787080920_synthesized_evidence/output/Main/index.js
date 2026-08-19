import * as Type_Proxy from "../Type.Proxy/index.js";

export const symbol = { reflectSymbol: $proxy => "alexandrite" }.reflectSymbol(Type_Proxy.Proxy);

export const reflectedString = { reflectType: $proxy => "reflected" }.reflectType(Type_Proxy.Proxy);

export const reflectedInteger = { reflectType: $proxy => 42 | 0 }.reflectType(Type_Proxy.Proxy);

export const reflectedTrue = { reflectType: $proxy => true }.reflectType(Type_Proxy.Proxy);

export const reflectedFalse = { reflectType: $proxy => false }.reflectType(Type_Proxy.Proxy);

export const reflectedLess = { reflectType: $proxy => ["LT"] }.reflectType(Type_Proxy.Proxy);

export const reflectedEqual = { reflectType: $proxy => ["EQ"] }.reflectType(Type_Proxy.Proxy);

export const reflectedGreater = { reflectType: $proxy => ["GT"] }.reflectType(Type_Proxy.Proxy);
