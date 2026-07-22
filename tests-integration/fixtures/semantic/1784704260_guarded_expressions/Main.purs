module Main where

data Maybe a = Just a | Nothing

recover :: forall a. a -> Maybe a -> a
recover fallback input
  | Just value <- input = value
  | true = fallback
