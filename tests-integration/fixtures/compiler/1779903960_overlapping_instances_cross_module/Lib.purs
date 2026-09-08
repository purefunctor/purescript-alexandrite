module Lib where

class C a b where
  test :: a -> b -> Int

data X = X

instance cxx :: C X a where
  test _ _ = 0
