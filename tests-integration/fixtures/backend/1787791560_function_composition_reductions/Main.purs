module Main where

import Control.Semigroupoid as Semigroupoid
import Lookalike as Lookalike

foreign import observe :: forall value. String -> value -> value
foreign import readTrace :: Boolean -> Array String

composed :: Int
composed = Semigroupoid.compose (\value -> value) (\value -> value) 42

composeOrder :: Boolean -> Int
composeOrder _ =
  Semigroupoid.compose
    (observe "outer" (\value -> value))
    (observe "inner" (\value -> value))
    (observe "argument" 42)

partiallyComposedOrder :: Boolean -> Int
partiallyComposedOrder _ =
  let
    composedFunction =
      Semigroupoid.compose
        (observe "outer" (\value -> value))
        (observe "inner" (\value -> value))
  in composedFunction (observe "argument" 42)

flippedComposed :: Int
flippedComposed = Semigroupoid.composeFlipped (\value -> value) (\value -> value) 42

flippedComposeOrder :: Boolean -> Int
flippedComposeOrder _ =
  Semigroupoid.composeFlipped
    (observe "inner" (\value -> value))
    (observe "outer" (\value -> value))
    (observe "argument" 42)

partiallyFlippedComposedOrder :: Boolean -> Int
partiallyFlippedComposedOrder _ =
  let
    composedFunction =
      Semigroupoid.composeFlipped
        (observe "inner" (\value -> value))
        (observe "outer" (\value -> value))
  in composedFunction (observe "argument" 42)

lookalikeCompose :: Int
lookalikeCompose = Lookalike.compose (\value -> value) (\value -> value) 42

lookalikeComposeFlipped :: Int
lookalikeComposeFlipped = Lookalike.composeFlipped (\value -> value) (\value -> value) 42
