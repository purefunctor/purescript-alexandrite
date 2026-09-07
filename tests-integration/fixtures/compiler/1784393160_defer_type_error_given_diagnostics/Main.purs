module Main where

import Prim.TypeError (class Fail, class Warn, Text)

class Value a where
  value :: a

instance Value Int where
  value = 42

guardedFailure :: Fail (Text "failure was demanded") => Int
guardedFailure = (value :: Fail (Text "failure was demanded") => Int)

useFailure :: Int
useFailure = guardedFailure

guardedWarning :: Warn (Text "warning was demanded") => Int
guardedWarning = (value :: Warn (Text "warning was demanded") => Int)

useWarning :: Int
useWarning = guardedWarning
