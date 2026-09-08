module Main where

import Data.Eq (class Eq, class Eq1)
import Library (ImportedOpaque)

foreign import data LocalOpaque :: Type -> Type

instance Eq a => Eq (LocalOpaque a) where
  eq _ _ = true

type LocalAlias = LocalOpaque

derive instance Eq1 LocalAlias

type ImportedAlias = ImportedOpaque

derive instance Eq1 ImportedAlias

foreign import data NoBase :: Type -> Type

derive instance Eq1 NoBase

data Product a b = Product a b

type ProductInt a = Product Int a

derive instance Eq1 ProductInt
