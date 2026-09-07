module Main where

import Data.Eq (class Eq, class Eq1)
import Data.Ord (class Ord, class Ord1)
import Data.Ordering (Ordering(..))

foreign import data Opaque :: Type -> Type

instance Eq a => Eq (Opaque a) where
  eq _ _ = true

derive instance Eq1 Opaque

instance Ord a => Ord (Opaque a) where
  compare _ _ = EQ

derive instance Ord1 Opaque
