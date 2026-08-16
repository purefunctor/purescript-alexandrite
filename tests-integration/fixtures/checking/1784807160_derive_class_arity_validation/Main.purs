module Main where

import Data.Bifoldable (class Bifoldable)
import Data.Bifunctor (class Bifunctor)
import Data.Bitraversable (class Bitraversable)
import Data.Eq (class Eq, class Eq1)
import Data.Foldable (class Foldable)
import Data.Functor (class Functor)
import Data.Functor.Contravariant (class Contravariant)
import Data.Ord (class Ord, class Ord1)
import Data.Profunctor (class Profunctor)
import Data.Traversable (class Traversable)

data Pair a b = Pair a b

derive instance Eq Pair Int
derive instance Ord Pair Int
derive instance Functor Pair Int
derive instance Bifunctor Pair Int
derive instance Foldable Pair Int
derive instance Bifoldable Pair Int
derive instance Traversable Pair Int
derive instance Bitraversable Pair Int
derive instance Contravariant Pair Int
derive instance Profunctor Pair Int
derive instance Eq1 Pair Int
derive instance Ord1 Pair Int
