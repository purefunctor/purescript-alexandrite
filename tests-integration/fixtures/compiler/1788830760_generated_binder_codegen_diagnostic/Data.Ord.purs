module Data.Ord where

import Data.Ordering (Ordering(..))

class Ord a where
  compare :: a -> a -> Ordering

instance ordInt :: Ord Int where
  compare _ _ = EQ 0
