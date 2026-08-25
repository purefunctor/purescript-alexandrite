module Main where

data Pair a b = Pair a b

identity :: forall a. a -> a
identity value = value

use :: Pair Int String
use = Pair (alias 42) (alias "x")
  where
  alias = identity
