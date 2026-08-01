module Main where

import Data.Functor (class Functor)
import Data.Functor.Contravariant (class Contravariant)

data Box a = Box a
derive instance Functor Box

data Predicate a = Predicate (a -> Boolean)
derive instance Contravariant Predicate

data BoxedInput a = BoxedInput (Predicate (Box a))
derive instance Contravariant BoxedInput
