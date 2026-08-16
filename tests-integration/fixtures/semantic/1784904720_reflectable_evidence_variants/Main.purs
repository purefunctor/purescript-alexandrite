module Main where

import Data.Reflectable (reflectType)
import Prim.Boolean (False, True)
import Prim.Ordering (EQ, GT, LT)
import Type.Proxy (Proxy(..))

symbol = reflectType (Proxy :: Proxy "symbol")

integer = reflectType (Proxy :: Proxy 42)

booleanTrue = reflectType (Proxy :: Proxy True)

booleanFalse = reflectType (Proxy :: Proxy False)

orderingLess = reflectType (Proxy :: Proxy LT)

orderingEqual = reflectType (Proxy :: Proxy EQ)

orderingGreater = reflectType (Proxy :: Proxy GT)
