module Main where

import Data.Functor.Contravariant (class Contravariant)
import Data.Profunctor (class Profunctor)

data Wrapped f a = Wrapped (f (a -> Int))
derive instance Contravariant (Wrapped f)

data Wrong f a b = Wrong (f a)
derive instance Profunctor (Wrong f)
