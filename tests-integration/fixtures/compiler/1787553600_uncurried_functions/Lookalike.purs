module Lookalike where

mkFn2 :: forall first second result. (first -> second -> result) -> first -> second -> result
mkFn2 function = function

runFn2 :: forall first second result. (first -> second -> result) -> first -> second -> result
runFn2 function = function
