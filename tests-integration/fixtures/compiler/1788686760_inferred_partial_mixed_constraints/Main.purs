module Main where

data Maybe a = Just a | Nothing

class Select a where
  select :: a -> a -> a

instance selectInt :: Select Int where
  select first second = second

mixed choice fallback =
  let
    Just value = choice
  in
    select value fallback

foreign import unsafePartial :: forall a. (Partial => a) -> a

discharged :: Maybe Int -> Int -> Int
discharged = unsafePartial mixed
