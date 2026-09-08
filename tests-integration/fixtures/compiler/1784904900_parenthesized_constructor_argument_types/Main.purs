module Main where

class Marker a

data Complex = Complex
  (Int -> Int)
  (forall value. value -> value)
  (Marker Int => Int)
  (Int :: Type)
