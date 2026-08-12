module Library where

import Data.Eq (class Eq)
import Data.Ord (class Ord)
import Data.Ordering (Ordering(..))

data ImportedStructural a = ImportedStructural a

data ImportedLifted a = ImportedLifted a

derive instance Eq a => Eq (ImportedLifted a)
derive instance Ord a => Ord (ImportedLifted a)

foreign import data ImportedOpaque :: Type -> Type

instance Eq a => Eq (ImportedOpaque a) where
  eq _ _ = true

instance Ord a => Ord (ImportedOpaque a) where
  compare _ _ = EQ
