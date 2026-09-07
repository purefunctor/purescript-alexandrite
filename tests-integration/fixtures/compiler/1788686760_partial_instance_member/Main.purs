module Main where

data Maybe a = Just a | Nothing

class Unwrap a where
  unwrap :: Partial => a -> Int

instance unwrapMaybe :: Unwrap (Maybe Int) where
  unwrap choice =
    let
      Just value = choice
    in
      value

foreign import unsafePartial :: forall a. (Partial => a) -> a

discharged :: Maybe Int -> Int
discharged = unsafePartial unwrap
