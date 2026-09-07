module Main where

import Control.Applicative (class Applicative, pure)
import Control.Apply (class Apply, apply)
import Control.Bind (class Bind, bind)
import Control.Monad (class Monad)
import Data.Functor (class Functor, map)

data Box value = Box value

liftApplicative
  :: forall functor value result
   . Applicative functor
  => (value -> result)
  -> functor value
  -> functor result
liftApplicative function value = apply (pure function) value

applyMonad
  :: forall monad value result
   . Monad monad
  => monad (value -> result)
  -> monad value
  -> monad result
applyMonad functions values =
  bind functions (\function -> bind values (\value -> pure (function value)))

instance functorBox :: Functor Box where
  map = liftApplicative

instance applyBox :: Apply Box where
  apply = applyMonad

instance applicativeBox :: Applicative Box where
  pure = Box

instance bindBox :: Bind Box where
  bind (Box value) continuation = continuation value

instance monadBox :: Monad Box

result :: Int
result = case map (\value -> value) (Box 42) of
  Box value -> value
