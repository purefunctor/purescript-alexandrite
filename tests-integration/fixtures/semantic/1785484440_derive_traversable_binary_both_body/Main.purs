module Main where

import Data.Functor (class Functor)
import Data.Bifunctor (class Bifunctor, bimap)
import Data.Foldable (class Foldable)
import Data.Bifoldable (class Bifoldable, bifoldl, bifoldr, bifoldMap)
import Data.Traversable (class Traversable)
import Data.Bitraversable (class Bitraversable)

data Duplicate p a = Duplicate (p a a)

instance Bifunctor p => Functor (Duplicate p) where
  map function (Duplicate value) = Duplicate (bimap function function value)

instance Bifoldable p => Foldable (Duplicate p) where
  foldr function initial (Duplicate value) = bifoldr function function initial value
  foldl function initial (Duplicate value) = bifoldl function function initial value
  foldMap function (Duplicate value) = bifoldMap function function value

derive instance Bitraversable p => Traversable (Duplicate p)
