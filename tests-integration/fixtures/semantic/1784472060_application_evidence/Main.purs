module Main where

class Constraint a

foreign import make :: Constraint String => Int -> String

instance constraintString :: Constraint String

test = make 0
