module Main where

class Default :: Type -> Constraint
class Default a

foreign import select :: forall a. Default a => (forall @b. a -> b -> a)

specialize :: forall a. Default a => a -> String -> a
specialize = select @String
