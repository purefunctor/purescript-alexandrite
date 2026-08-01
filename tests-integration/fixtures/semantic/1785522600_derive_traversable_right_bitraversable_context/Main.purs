module Main where

import Data.Bifoldable (class Bifoldable, bifoldl, bifoldMap, bifoldr)
import Data.Bifunctor (class Bifunctor, bimap)
import Data.Bitraversable (class Bitraversable)
import Data.Foldable (class Foldable)
import Data.Functor (class Functor)
import Data.Monoid (mempty)
import Data.Traversable (class Traversable)

data RightNested p a = RightNested (p Int a)

instance Bifunctor p => Functor (RightNested p) where
  map function (RightNested value) =
    RightNested (bimap (\fixed -> fixed) function value)

instance Bifoldable p => Foldable (RightNested p) where
  foldr function initial (RightNested value) =
    bifoldr (\_ accumulated -> accumulated) function initial value
  foldl function initial (RightNested value) =
    bifoldl (\accumulated _ -> accumulated) function initial value
  foldMap function (RightNested value) = bifoldMap (\_ -> mempty) function value

derive instance Bitraversable p => Traversable (RightNested p)
