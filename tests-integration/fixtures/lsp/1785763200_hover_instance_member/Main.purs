module Main where

import Data.Functor (class Functor)

newtype Continuation r a = Continuation ((a -> r) -> r)

instance Functor (Continuation r) where
  map :: forall a b. (a -> b) -> Continuation r a -> Continuation r b
-- $
  map function (Continuation program) =
-- $
    Continuation \return ->
      program \a ->
        return (function a)

value :: Int
value = 42
-- $
