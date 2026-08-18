module Main where

import Data.Ordering (Ordering)
import Data.Reflectable (reflectType)
import Data.Symbol (reflectSymbol)
import Prim.Boolean (False, True)
import Prim.Ordering (EQ, GT, LT)
import Type.Proxy (Proxy(..))

symbol :: String
symbol = reflectSymbol (Proxy :: Proxy "alexandrite")

reflectedString :: String
reflectedString = reflectType (Proxy :: Proxy "reflected")

reflectedInteger :: Int
reflectedInteger = reflectType (Proxy :: Proxy 42)

reflectedTrue :: Boolean
reflectedTrue = reflectType (Proxy :: Proxy True)

reflectedFalse :: Boolean
reflectedFalse = reflectType (Proxy :: Proxy False)

reflectedLess :: Ordering
reflectedLess = reflectType (Proxy :: Proxy LT)

reflectedEqual :: Ordering
reflectedEqual = reflectType (Proxy :: Proxy EQ)

reflectedGreater :: Ordering
reflectedGreater = reflectType (Proxy :: Proxy GT)
