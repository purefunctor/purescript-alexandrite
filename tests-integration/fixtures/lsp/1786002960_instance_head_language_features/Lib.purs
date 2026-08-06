module Lib where

class Functor f where
  map :: forall a b. (a -> b) -> f a -> f b

class Collision f where
  collide :: forall a. f a -> a

class Original f where
  original :: forall a. f a -> a

instance Partial
