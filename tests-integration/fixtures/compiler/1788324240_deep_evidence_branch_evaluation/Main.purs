module Main where

import Prim.Int (class Add)

data Proxy :: forall k. k -> Type
data Proxy a = Proxy

class Build (number :: Int) where
  built :: Int

instance buildZero :: Build 0 where
  built = 0
else instance buildNext :: (Add previous 1 current, Build previous) => Build current where
  built = crash 0

foreign import crash :: Int -> Int

buildValue :: forall number. Build number => Proxy number -> Int
buildValue _ = built @number

evaluateIf :: Boolean -> Int
evaluateIf condition =
  if condition then buildValue (Proxy :: Proxy 70) else 0

evaluateCase :: Boolean -> Int
evaluateCase condition = case condition of
  true -> buildValue (Proxy :: Proxy 70)
  false -> 0

evaluateGuard :: Boolean -> Int
evaluateGuard condition
  | condition = buildValue (Proxy :: Proxy 70)
  | true = 0
