module Main where

import Data.Functor (class Functor)
import Data.Functor.Contravariant (class Contravariant)

data Box a = Box a
derive instance Functor Box

data BoxedPredicate a = BoxedPredicate (Box (a -> Boolean))
derive instance Contravariant BoxedPredicate
