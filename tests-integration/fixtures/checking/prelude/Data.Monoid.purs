module Data.Monoid where

import Data.Semigroup (class Semigroup)

class Semigroup m <= Monoid m where
  mempty :: m
