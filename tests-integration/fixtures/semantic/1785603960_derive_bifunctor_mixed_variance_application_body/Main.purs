module Main where

import Data.Bifunctor (class Bifunctor)
import Data.Functor.Contravariant (class Contravariant)
import Data.Profunctor (class Profunctor)

data Predicate a = Predicate (a -> Boolean)

derive instance Contravariant Predicate

data Function a b = Function (a -> b)

derive instance Profunctor Function

data Mixed a b = Mixed (Function (Predicate a) b)

derive instance Bifunctor Mixed
