module Main (Data, Newtype, Synonym, class Example, Foreign, foreignValue) where

data Data :: Type -> Type
data Data a = Data a
type role Data representational

newtype Newtype :: Type -> Type
newtype Newtype a = Newtype a
type role Newtype representational

type Synonym :: Type -> Type
type Synonym a = a

class Example :: Type -> Constraint
class Example a where
  example :: a -> a

foreign import data Foreign :: Type -> Type
type role Foreign representational

foreign import foreignValue :: Foreign Int
