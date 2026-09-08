module Main where

data Maybe a = Just a | Nothing

nothing :: forall a. Maybe a
nothing = Nothing
