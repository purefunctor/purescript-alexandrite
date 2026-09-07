module Main where

import Data.Functor (class Functor)
import Data.Bifunctor (class Bifunctor)
import Data.Foldable (class Foldable)
import Data.Bifoldable (class Bifoldable)
import Data.Traversable (class Traversable)
import Data.Bitraversable (class Bitraversable)

data Wrap f g a b = Wrap (f a) (g b)
derive instance (Functor f, Functor g) => Bifunctor (Wrap f g)
derive instance (Foldable f, Foldable g) => Bifoldable (Wrap f g)
derive instance (Traversable f, Traversable g) => Bitraversable (Wrap f g)
