module Main where

import Data.Bifunctor (class Bifunctor)
import Data.Functor (class Functor)
import Data.Functor.Contravariant (class Contravariant)
import Data.Profunctor (class Profunctor)

data Predicate a = Predicate (a -> Boolean)
derive instance Contravariant Predicate

data UnaryContravariant a = UnaryContravariant (Predicate a)
derive instance Contravariant UnaryContravariant

data Box a = Box a
derive instance Functor Box

data UnaryCovariant a = UnaryCovariant (Box (a -> Boolean))
derive instance Contravariant UnaryCovariant

data Pair a b = Pair a b
derive instance Bifunctor Pair
derive instance Functor (Pair a)

data BinaryFirst a = BinaryFirst (Pair (a -> Boolean) Int)
derive instance Contravariant BinaryFirst

data BinarySecond a = BinarySecond (Pair Int (a -> Boolean))
derive instance Contravariant BinarySecond

data BinaryBoth a = BinaryBoth (Pair (a -> Boolean) (a -> String))
derive instance Contravariant BinaryBoth

data FunctionLike a b = FunctionLike (a -> b)
derive instance Profunctor FunctionLike

data NestedProfunctor a b = NestedProfunctor (FunctionLike a b)
derive instance Profunctor NestedProfunctor
