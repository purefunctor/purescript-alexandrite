module Main where

class Missing a where
  missing :: a

bad :: Int
bad = (missing :: Missing Int => Int)

class Supplied a where
  supplied :: a

good :: Supplied Int => Int
good = (supplied :: Supplied Int => Int)
