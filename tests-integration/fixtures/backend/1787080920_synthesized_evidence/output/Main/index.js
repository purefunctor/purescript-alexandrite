import * as Type_Proxy from "../Type.Proxy/index.js";

export const symbol = (0, { reflectSymbol: $proxy => "alexandrite" }.reflectSymbol)(
  Type_Proxy.Proxy
);

export const reflectedString = (0, { reflectType: $proxy => "reflected" }.reflectType)(
  Type_Proxy.Proxy
);

export const reflectedInteger = (0, { reflectType: $proxy => 42 | 0 }.reflectType)(
  Type_Proxy.Proxy
);

export const reflectedTrue = (0, { reflectType: $proxy => true }.reflectType)(Type_Proxy.Proxy);

export const reflectedFalse = (0, { reflectType: $proxy => false }.reflectType)(Type_Proxy.Proxy);

export const reflectedLess = (0, { reflectType: $proxy => ["LT"] }.reflectType)(Type_Proxy.Proxy);

export const reflectedEqual = (0, { reflectType: $proxy => ["EQ"] }.reflectType)(Type_Proxy.Proxy);

export const reflectedGreater = (0, { reflectType: $proxy => ["GT"] }.reflectType)(
  Type_Proxy.Proxy
);
