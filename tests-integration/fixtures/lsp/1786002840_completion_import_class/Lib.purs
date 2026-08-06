module Lib where

class Functor f where
  map :: forall a b. (a -> b) -> f a -> f b

class Foldable f where
  fold :: forall a. f a -> a

data Furniture
