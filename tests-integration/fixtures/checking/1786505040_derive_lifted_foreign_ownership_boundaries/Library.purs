module Library where

import Data.Eq (class Eq)

foreign import data ImportedOpaque :: Type -> Type

instance Eq a => Eq (ImportedOpaque a) where
  eq _ _ = true
