module Main where

data Maybe a = Just a | Nothing

inferred choice =
  let
    Just value = choice
  in
    value

signed :: forall a. Partial => Maybe a -> a
signed choice = inferred choice

foreign import unsafePartial :: forall a. (Partial => a) -> a

discharged :: Maybe Int -> Int
discharged = unsafePartial inferred
