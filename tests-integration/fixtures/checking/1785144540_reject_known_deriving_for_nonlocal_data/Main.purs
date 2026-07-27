module Main where

import Data.Eq (class Eq, class Eq1)
import Data.Ord (class Ord, class Ord1)
import Library (Imported)

foreign import data Opaque1 :: Type -> Type

derive instance Eq1 Opaque1
derive instance Ord1 Opaque1

derive instance Eq a => Eq (Imported a)
derive instance Eq1 Imported
derive instance Ord a => Ord (Imported a)
derive instance Ord1 Imported
