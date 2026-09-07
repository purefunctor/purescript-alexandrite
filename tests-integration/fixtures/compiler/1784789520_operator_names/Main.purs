module Main where

class Combine :: Type -> Constraint
class Combine a where
  combine :: a -> a -> a

infixr 5 combine as <>

operatorName :: forall a. Combine a => a -> a -> a
operatorName = (<>)

operatorApplication :: forall a. Combine a => a -> a -> a
operatorApplication left right = (<>) left right

data List a = Cons a (List a) | Nil

infixr 5 Cons as :

constructorOperator = (:)
