import * as Data_Reflectable from "../Data.Reflectable/index.js";
import * as Data_Symbol from "../Data.Symbol/index.js";
import * as Type_Proxy from "../Type.Proxy/index.js";

export const symbol = Data_Symbol.reflectSymbol({ reflectSymbol: $proxy => "alexandrite" })(
  Type_Proxy.Proxy
);

export const reflectedString = Data_Reflectable.reflectType({ reflectType: $proxy => "reflected" })(
  Type_Proxy.Proxy
);

export const reflectedInteger = Data_Reflectable.reflectType({ reflectType: $proxy => 42 | 0 })(
  Type_Proxy.Proxy
);

export const reflectedTrue = Data_Reflectable.reflectType({ reflectType: $proxy => true })(
  Type_Proxy.Proxy
);

export const reflectedFalse = Data_Reflectable.reflectType({ reflectType: $proxy => false })(
  Type_Proxy.Proxy
);

export const reflectedLess = Data_Reflectable.reflectType({ reflectType: $proxy => ["LT"] })(
  Type_Proxy.Proxy
);

export const reflectedEqual = Data_Reflectable.reflectType({ reflectType: $proxy => ["EQ"] })(
  Type_Proxy.Proxy
);

export const reflectedGreater = Data_Reflectable.reflectType({ reflectType: $proxy => ["GT"] })(
  Type_Proxy.Proxy
);
