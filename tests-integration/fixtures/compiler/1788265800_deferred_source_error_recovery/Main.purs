module Main where

usable :: Int
usable = 42

deferred :: Int -> Int
deferred _ = missing

nested :: Int -> Int -> Int
nested _ _ = missing
