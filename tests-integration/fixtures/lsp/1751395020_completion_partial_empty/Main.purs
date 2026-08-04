module Main where

life = 42

data Maybe a = Just a | Nothing

local (Just first) { second } =
--                               ^
