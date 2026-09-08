module Main where

identity :: forall a. a -> a
identity value = value

apply :: forall a b. (a -> b) -> a -> b
apply function argument = function argument

orphan = _

orphanInLambda = \value -> _

failedBody = apply _ missing

invalidExpected :: Int
invalidExpected = identity _
