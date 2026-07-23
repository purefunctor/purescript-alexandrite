module Main where

apply :: forall a b. (a -> b) -> a -> b
apply function value = function value

argument = apply \value -> value

functionPosition = (\function -> function) apply
