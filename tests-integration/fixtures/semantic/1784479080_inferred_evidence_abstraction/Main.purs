module Main where

class Constraint a

foreign import make :: forall a. Constraint a => a -> String

test value = make value
