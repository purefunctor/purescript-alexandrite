module Main where

data Maybe a = Just a | Nothing

fromMaybe default (Just value) = value
fromMaybe default Nothing = default
