module Main where

import Prim.Int (class Add)

class Seed

instance seed :: Seed

class Left number where
  left :: Int

instance leftZero :: Seed => Left 0 where
  left = observe "left"
else instance leftNext :: (Add previous 1 current, Left previous) => Left current where
  left = 0

class Right number where
  right :: Int

instance rightZero :: Seed => Right 0 where
  right = observe "right"
else instance rightNext :: (Add previous 1 current, Right previous) => Right current where
  right = 0

class Combined where
  combined :: Int

instance combinedInstance :: (Left 70, Right 40) => Combined where
  combined = 0

foreign import observe :: String -> Int

result :: Int
result = combined
