module Main where

import Data.Bifunctor (class Bifunctor)
import Data.Functor (class Functor)

data Either a b = Left a | Right b
derive instance Bifunctor Either

data Pair a b = Pair a b
derive instance Bifunctor Pair

data Const2 value a b = Const2 value
derive instance Bifunctor (Const2 value)

data WrapBoth f g a b = WrapBoth (f a) (g b)
derive instance (Functor f, Functor g) => Bifunctor (WrapBoth f g)

data Tuple a b = Tuple a b
derive instance Bifunctor Tuple
derive instance Functor (Tuple a)

data Both a b = Both (Tuple a b)
derive instance Bifunctor Both

data OneSided a b = OneSidedFirst (Tuple a Int) | OneSidedSecond (Tuple Int b)
derive instance Bifunctor OneSided

data Box a = Box a
derive instance Functor Box

data Nested a b = Nested (Box (Tuple a b))
derive instance Bifunctor Nested

data Triple fixed a b = Triple fixed a b
derive instance Bifunctor (Triple fixed)
derive instance Functor (Triple fixed a)

data NestedTriple a b = NestedTriple (Triple Int a b)
derive instance Bifunctor NestedTriple

data NestedTripleLast a b = NestedTripleLast (Triple Int String b)
derive instance Bifunctor NestedTripleLast

data ReaderPair a b = ReaderPair (Int -> Tuple a b)
derive instance Bifunctor ReaderPair

data RecordPair a b = RecordPair { first :: a, second :: b, fixed :: Int }
derive instance Bifunctor RecordPair
