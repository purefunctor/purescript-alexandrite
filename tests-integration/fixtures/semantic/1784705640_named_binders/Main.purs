module Main where

data Maybe a = Just a | Nothing

fromMaybe :: forall a. a -> Maybe a -> a
fromMaybe fallback whole@(Just value) = value
fromMaybe fallback Nothing = fallback
