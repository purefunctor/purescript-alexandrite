module Main where

data Maybe a = Just a | Nothing

test =
  let
    Just x = Just 1
  in
    x

foreign import unsafePartial :: forall a. (Partial => a) -> a

consumed :: Partial => Int
consumed = test

discharged :: Int
discharged = unsafePartial test
