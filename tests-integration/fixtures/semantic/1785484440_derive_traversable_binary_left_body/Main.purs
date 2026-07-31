module Main where

import Data.Functor (class Functor)
import Data.Bifunctor (class Bifunctor, bimap)
import Data.Foldable (class Foldable)
import Data.Bifoldable (class Bifoldable, bifoldl, bifoldr)
import Data.Traversable (class Traversable)
import Data.Bitraversable (class Bitraversable)

data LeftDuplicate p a = LeftDuplicate (p a Int)

instance Bifunctor p => Functor (LeftDuplicate p) where
  map function (LeftDuplicate value) =
    LeftDuplicate (bimap function (\fixed -> fixed) value)

instance Bifoldable p => Foldable (LeftDuplicate p) where
  foldr function initial (LeftDuplicate value) =
    bifoldr function (\_ accumulated -> accumulated) initial value
  foldl function initial (LeftDuplicate value) =
    bifoldl function (\accumulated _ -> accumulated) initial value

derive instance Bitraversable p => Traversable (LeftDuplicate p)
