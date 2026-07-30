module Main where

import Data.Functor (class Functor)

data Identity a = Identity a
derive instance Functor Identity

data Const e a = Const e
derive instance Functor (Const e)

data Maybe a = Nothing | Just a
derive instance Functor Maybe

data Wrap f a = Wrap (f a)
derive instance Functor f => Functor (Wrap f)

data Compose f g a = Compose (f (g a))
derive instance (Functor f, Functor g) => Functor (Compose f g)

data Reader r a = Reader (r -> a)
derive instance Functor (Reader r)

data NestedReader r s a = NestedReader (r -> s -> a)
derive instance Functor (NestedReader r s)

data Cont r a = Cont ((a -> r) -> r)
derive instance Functor (Cont r)

data Record a = Record { changed :: a, fixed :: Int }
derive instance Functor Record

data OpenRecord r a = OpenRecord { changed :: a | r }
derive instance Functor (OpenRecord r)
