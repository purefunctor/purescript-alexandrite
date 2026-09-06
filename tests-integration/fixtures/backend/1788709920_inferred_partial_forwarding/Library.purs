module Library where

data Maybe a = Just a | Nothing

unwrap :: forall a. Partial => Maybe a -> a
unwrap choice =
  let
    Just value = choice
  in
    value
