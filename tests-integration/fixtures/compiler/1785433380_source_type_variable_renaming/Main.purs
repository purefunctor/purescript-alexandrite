module Main where

data Proxy :: forall k. k -> Type
data Proxy value = Proxy

identity :: forall @value. value -> value
identity value = value

class Grouped :: Type -> Type -> Constraint
class Grouped left right where
  grouped :: forall (value :: Type). Proxy value -> Proxy right -> Proxy value
  ranked :: (forall (value :: Type). Proxy value -> Proxy value) -> Proxy right

instance Grouped (function left) (function right) where
  grouped :: forall (left :: Type). Proxy left -> Proxy (function right) -> Proxy left
  grouped (value :: Proxy left) (_ :: Proxy (function right)) =
    identity @(Proxy left) (value :: Proxy left)

  ranked :: (forall (left :: Type). Proxy left -> Proxy left) -> Proxy (function right)
  ranked (_ :: forall (left :: Type). Proxy left -> Proxy left) = Proxy
