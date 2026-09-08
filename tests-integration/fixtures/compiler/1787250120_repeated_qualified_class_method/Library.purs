module Library where

import Data.Eq (class Eq)

class Eq a <= Ordered a where
  lessThanOrEqual :: a -> a -> Boolean

instance Ordered Int where
  lessThanOrEqual _ _ = true
